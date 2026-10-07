//! Common intent has one reference; plans, disk operations and evaluations remain device-local.
use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeLink {
    kind: String,
    schema: u8,
    common_change_ref: String,
    common_baseline: CommonBaseline,
    device_sha256: String,
    proposal_id: String,
    plan_sha256: String,
    target_ids: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleLink {
    kind: String,
    schema: u8,
    common_change_ref: String,
    proposal_id: String,
    plan_sha256: String,
    bundle_sha256: String,
    payload_sha256: String,
}

fn validate_link(link: &ChangeLink, plan: &Plan, plan_sha256: &str) -> Result<(), ContractError> {
    contract(
        &link.kind,
        link.schema,
        "groundline-environment-change-link",
    )?;
    let local = link.common_baseline.local(&plan.exception_revision);
    validate_baseline(&local)?;
    let mut targets = plan
        .entries
        .iter()
        .map(|e| e.target_id.clone())
        .collect::<Vec<_>>();
    targets.sort();
    if link.common_change_ref != link.common_baseline.digest()?
        || hash(&bytes(&local)?) != plan.baseline_sha256
        || link.device_sha256 != hash(plan.device_id.as_bytes())
        || link.proposal_id != plan.proposal_id
        || link.plan_sha256 != plan_sha256
        || link.target_ids != targets
    {
        return Err(error("change_link_binding_mismatch"));
    }
    Ok(())
}

pub(super) fn bind_plan(
    store: &Store,
    current: &Current,
    plan: &Plan,
    plan_sha256: &str,
) -> Result<String, ContractError> {
    let common = CommonBaseline::from_local(&current.baseline);
    let mut target_ids = plan
        .entries
        .iter()
        .map(|e| e.target_id.clone())
        .collect::<Vec<_>>();
    target_ids.sort();
    let link = ChangeLink {
        kind: "groundline-environment-change-link".into(),
        schema: 1,
        common_change_ref: common.digest()?,
        common_baseline: common,
        device_sha256: hash(plan.device_id.as_bytes()),
        proposal_id: plan.proposal_id.clone(),
        plan_sha256: plan_sha256.into(),
        target_ids,
    };
    validate_link(&link, plan, plan_sha256)?;
    store.artifacts("change-links", true)?.write(
        &format!("{}.json", plan.proposal_id),
        &bytes(&link)?,
        false,
    )?;
    Ok(link.common_change_ref)
}

pub(super) fn bind_bundle(
    store: &Store,
    plan: &Plan,
    plan_sha256: &str,
    bundle_sha256: &str,
    bundle: &Bundle,
) -> Result<(), ContractError> {
    let link = read_link(store, plan, plan_sha256)?.ok_or_else(|| error("change_link_missing"))?;
    if link.common_baseline != *bundle.baseline() || !valid_hash(bundle_sha256) {
        return Err(error("bundle_change_binding_mismatch"));
    }
    let record = BundleLink {
        kind: "groundline-environment-bundle-link".into(),
        schema: 1,
        common_change_ref: link.common_change_ref,
        proposal_id: plan.proposal_id.clone(),
        plan_sha256: plan_sha256.into(),
        bundle_sha256: bundle_sha256.into(),
        payload_sha256: bundle.payload_sha256.clone(),
    };
    store.artifacts("bundle-links", true)?.write(
        &format!("{}.json", plan.proposal_id),
        &bytes(&record)?,
        false,
    )
}

fn read_link(
    store: &Store,
    plan: &Plan,
    plan_sha256: &str,
) -> Result<Option<ChangeLink>, ContractError> {
    let Some(directory) = store.root.optional_child("change-links")? else {
        return Ok(None);
    };
    let Some(read) = directory.read(&format!("{}.json", plan.proposal_id))? else {
        return Ok(None);
    };
    let link: ChangeLink = parse(&read.bytes)?;
    validate_link(&link, plan, plan_sha256)?;
    Ok(Some(link))
}

pub(super) fn check_link(
    store: &Store,
    plan: &Plan,
    plan_sha256: &str,
) -> Result<(), ContractError> {
    if let Some(link) = read_link(store, plan, plan_sha256)? {
        bundle_link(store, &link)?;
    }
    Ok(())
}

fn bundle_link(store: &Store, link: &ChangeLink) -> Result<Option<Value>, ContractError> {
    let Some(directory) = store.root.optional_child("bundle-links")? else {
        return Ok(None);
    };
    let Some(read) = directory.read(&format!("{}.json", link.proposal_id))? else {
        return Ok(None);
    };
    let record: BundleLink = parse(&read.bytes)?;
    contract(
        &record.kind,
        record.schema,
        "groundline-environment-bundle-link",
    )?;
    if record.proposal_id != link.proposal_id
        || record.plan_sha256 != link.plan_sha256
        || record.common_change_ref != link.common_change_ref
        || !valid_hash(&record.bundle_sha256)
        || !valid_hash(&record.payload_sha256)
    {
        return Err(error("bundle_change_binding_mismatch"));
    }
    Ok(Some(
        json!({"bundle_sha256":record.bundle_sha256,"payload_sha256":record.payload_sha256}),
    ))
}

pub(super) fn status(
    state_dir: &Path,
    learning_state: Option<&Path>,
) -> Result<Value, ContractError> {
    let store = Store::open(state_dir, false)?;
    let _lock = store.root.lock()?;
    let current = store.current()?;
    let mut rows = vec![];
    let mut unlinked_plan_count = 0;
    let mut operations = BTreeMap::new();
    if let Some(directory) = store.root.optional_child("operations")? {
        for name in directory.names()? {
            if !name.ends_with(".json") {
                continue;
            }
            let read = directory
                .read(&name)?
                .ok_or_else(|| error("operation_missing"))?;
            let operation: Operation = parse(&read.bytes)?;
            check_operation(&operation)?;
            if name != format!("{}.json", operation.operation_id) {
                return Err(error("operation_id_mismatch"));
            }
            operations.insert(operation.operation_id.clone(), (operation, read.bytes));
        }
    }
    if let Some(directory) = store.root.optional_child("plans")? {
        for name in directory.names()? {
            let Some(proposal_id) = name.strip_suffix(".json") else {
                continue;
            };
            let (plan, plan_sha256) = store.plan(proposal_id)?;
            let Some(link) = read_link(&store, &plan, &plan_sha256)? else {
                unlinked_plan_count += 1;
                continue;
            };
            let mut local_operations = vec![];
            for (operation, raw) in operations
                .values()
                .filter(|(op, _)| op.plan_sha256 == plan_sha256)
            {
                if operation.device_id != plan.device_id
                    || operation.proposal_id != plan.proposal_id
                {
                    return Err(error("change_operation_binding_mismatch"));
                }
                plan_for_observed_operation(&store, operation)?;
                let operation_sha256 = hash(raw);
                let evaluations = if let Some(path) = learning_state {
                    crate::learning::application_evaluations(
                        path,
                        &serde_json::to_value(operation).map_err(|_| error("serialization"))?,
                        &operation_sha256,
                    )?
                } else {
                    json!({"evaluations":[],"effect_verified":false})
                };
                local_operations.push(json!({"operation_id":operation.operation_id,
                    "operation_sha256":operation_sha256,"action":operation.action,"status":operation.status,
                    "native_activation":operation.native_activation,"evaluations":evaluations["evaluations"],
                    "effect_verified":false}));
            }
            rows.push(json!({"common_change_ref":link.common_change_ref,"device_sha256":link.device_sha256,
                "proposal_id":link.proposal_id,"plan_sha256":link.plan_sha256,"target_ids":link.target_ids,
                "bundle":bundle_link(&store, &link)?,"operations":local_operations,
                "native_activation":"UNVERIFIED","effect_verified":false}));
        }
    }
    let mut result = output("status");
    result["status"] = json!("OBSERVATIONAL");
    result["device_sha256"] = json!(hash(current.registry.device_id.as_bytes()));
    result["common_change_ref"] = json!(CommonBaseline::from_local(&current.baseline).digest()?);
    result["desired_revision"] = json!(current.head.revision);
    result["exception_revision"] = json!(current.baseline.exception_revision);
    result["changes"] = json!(rows);
    result["unlinked_plan_count"] = json!(unlinked_plan_count);
    result["other_devices_verified"] = json!(false);
    result["mutation_performed"] = json!(false);
    Ok(result)
}
