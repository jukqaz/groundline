//! Identify literal, unconditionally awaited native calls. Never execute JS.
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, ObjectPropertyKind, Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::tool_outcome::handle_key;

#[derive(Clone)]
pub(super) struct Slot {
    pub poll: Option<String>,
    pub verification: bool,
}

pub(super) struct Plan {
    pub slots: Vec<Slot>,
    pub unobserved_call: bool,
}

fn native_key(name: &str, values: &Map<String, Value>) -> Option<String> {
    let (kind, field, allowed): (_, _, &[&str]) = match name {
        "write_stdin" => (
            "process",
            "session_id",
            &["session_id", "chars", "yield_time_ms", "max_output_tokens"],
        ),
        "wait" => (
            "cell",
            "cell_id",
            &["cell_id", "yield_time_ms", "max_tokens", "terminate"],
        ),
        _ => return None,
    };
    if values.keys().any(|key| !allowed.contains(&key.as_str()))
        || values.get("chars").is_some_and(|v| v.as_str() != Some(""))
        || values
            .get("terminate")
            .is_some_and(|v| v.as_bool() != Some(false))
    {
        return None;
    }
    let value = values.get(field)?;
    let id = value
        .as_u64()
        .map(|n| n.to_string())
        .or_else(|| value.as_str().map(str::to_owned))?;
    handle_key(kind, &id)
}

fn literal(value: &Expression<'_>) -> Option<Value> {
    match value {
        Expression::StringLiteral(v) => Some(Value::from(v.value.as_str())),
        Expression::NumericLiteral(v)
            if v.value >= 0.0 && v.value <= 9_007_199_254_740_991.0 && v.value.fract() == 0.0 =>
        {
            Some(Value::from(v.value as u64))
        }
        Expression::BooleanLiteral(v) => Some(Value::from(v.value)),
        Expression::NullLiteral(_) => Some(Value::Null),
        _ => None,
    }
}

fn poll(expression: &Expression<'_>) -> Option<String> {
    let (name, values) = native_call(expression)?;
    native_key(&name, &values)
}

fn native_call(expression: &Expression<'_>) -> Option<(String, Map<String, Value>)> {
    let Expression::AwaitExpression(awaited) = expression else {
        return None;
    };
    let Expression::CallExpression(call) = &awaited.argument else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return None;
    };
    if !member.object.is_specific_id("tools")
        || member.optional
        || call.optional
        || call.arguments.len() != 1
    {
        return None;
    }
    let Argument::ObjectExpression(object) = &call.arguments[0] else {
        return None;
    };
    let mut values = Map::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property.method || property.shorthand {
            return None;
        }
        let key = property.key.static_name()?.into_owned();
        if values.insert(key, literal(&property.value)?).is_some() {
            return None;
        }
    }
    Some((member.property.name.to_string(), values))
}

fn slot(expression: &Expression<'_>) -> Option<Slot> {
    let (name, values) = native_call(expression)?;
    Some(Slot {
        poll: native_key(&name, &values),
        verification: name == "exec_command"
            && values
                .get("cmd")
                .and_then(Value::as_str)
                .is_some_and(|cmd| super::tool_category("exec_command", cmd) == "verification"),
    })
}

/// Map an exact sequence of top-level text emissions to native calls. Multiple
/// polls can coexist with inspections without borrowing their success status.
pub(super) fn plan(name: &str, arguments: &str) -> Option<Plan> {
    if name != "exec"
        || arguments.len() > 8192
        || arguments
            .bytes()
            .filter(|b| !b.is_ascii_alphanumeric() && !b.is_ascii_whitespace() && *b != b'_')
            .count()
            > 256
    {
        return None;
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, arguments, SourceType::mjs()).parse();
    if !parsed.diagnostics.is_empty() || parsed.fatal_error || !parsed.program.directives.is_empty()
    {
        return None;
    }
    let mut variables = BTreeMap::new();
    let mut slots = Vec::new();
    for statement in &parsed.program.body {
        match statement {
            Statement::VariableDeclaration(declaration) if declaration.declarations.len() == 1 => {
                let declaration = &declaration.declarations[0];
                let BindingPattern::BindingIdentifier(binding) = &declaration.id else {
                    return None;
                };
                if matches!(binding.name.as_str(), "text" | "tools") {
                    return None;
                }
                if variables
                    .insert(
                        binding.name.to_string(),
                        (slot(declaration.init.as_ref()?)?, false),
                    )
                    .is_some()
                {
                    return None;
                }
            }
            Statement::ExpressionStatement(statement) => {
                let call = text_call(&statement.expression)?;
                let argument = call.arguments[0].as_expression()?;
                let emission = if let Expression::Identifier(id) = argument {
                    let (slot, observed) = variables.get_mut(id.name.as_str())?;
                    *observed = true;
                    slot.clone()
                } else if literal(argument).is_some() {
                    Slot {
                        poll: None,
                        verification: false,
                    }
                } else {
                    slot(argument)?
                };
                slots.push(emission);
            }
            _ => return None,
        }
    }
    (!slots.is_empty() && slots.len() <= 128).then(|| Plan {
        slots,
        unobserved_call: variables.values().any(|(_, observed)| !observed),
    })
}

fn text_call<'a, 'b>(expression: &'b Expression<'a>) -> Option<&'b CallExpression<'a>> {
    let Expression::CallExpression(call) = expression else {
        return None;
    };
    (call.callee.is_specific_id("text") && !call.optional && call.arguments.len() == 1)
        .then_some(call)
}

pub(super) fn key(name: &str, arguments: &str) -> Option<String> {
    if matches!(name, "wait" | "write_stdin") {
        return native_key(
            name,
            serde_json::from_str::<Value>(arguments).ok()?.as_object()?,
        );
    }
    if name != "exec" || !arguments.contains("write_stdin") {
        return None;
    }
    // Reject complex programs before parsing: this is a proof of a tiny polling
    // wrapper, not general static execution or an unbounded transcript parser.
    if arguments.len() > 8192
        || arguments
            .bytes()
            .filter(|b| !b.is_ascii_alphanumeric() && !b.is_ascii_whitespace() && *b != b'_')
            .count()
            > 256
    {
        return None;
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, arguments, SourceType::mjs()).parse();
    if !parsed.diagnostics.is_empty() || parsed.fatal_error || !parsed.program.directives.is_empty()
    {
        return None;
    }
    match parsed.program.body.as_slice() {
        [Statement::ExpressionStatement(statement)] => {
            if let Some(call) = text_call(&statement.expression) {
                poll(call.arguments[0].as_expression()?)
            } else {
                poll(&statement.expression)
            }
        }
        [
            Statement::VariableDeclaration(declaration),
            Statement::ExpressionStatement(statement),
        ] if declaration.declarations.len() == 1 => {
            let variable = &declaration.declarations[0];
            let BindingPattern::BindingIdentifier(binding) = &variable.id else {
                return None;
            };
            if matches!(binding.name.as_str(), "tools" | "text") {
                return None;
            }
            let output = text_call(&statement.expression)?;
            if !output.arguments[0]
                .as_expression()?
                .is_specific_id(binding.name.as_str())
            {
                return None;
            }
            poll(variable.init.as_ref()?)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plans_exact_emissions_and_detects_unobserved_calls() {
        let source = "const a = await tools.exec_command({cmd:'cargo test'}); text('label'); text(await tools.write_stdin({session_id:5})); text(a);";
        let p = plan("exec", source).unwrap();
        assert_eq!(p.slots.len(), 3);
        assert!(!p.unobserved_call);
        assert!(p.slots[2].verification);
        assert_eq!(p.slots[1].poll, handle_key("process", "5"));
        assert!(plan("exec", "const a = await tools.exec_command({cmd:'cargo test'}); text(await tools.exec_command({cmd:'pwd'}));").unwrap().unobserved_call);
        for source in [
            "if (true) text(await tools.exec_command({cmd:'cargo test'}));",
            "const a = await tools.exec_command({cmd:'cargo test'}); a = {}; text(a);",
            "text(await tools.exec_command({cmd:command}));",
            "text(await tools.exec_command({get cmd() { return 'cargo test' }}));",
        ] {
            assert!(plan("exec", source).is_none());
        }
    }
    #[test]
    fn accepts_literal_direct_and_single_awaited_polls() {
        let expected = handle_key("process", "123").unwrap();
        for source in [
            "text(await tools.write_stdin({session_id:123,chars:'',yield_time_ms:1000}));",
            "const r = await tools.write_stdin({session_id:123}); text(r)",
            "// poll\nawait tools.write_stdin({session_id:123});",
        ] {
            assert_eq!(key("exec", source).as_deref(), Some(expected.as_str()));
        }
        assert_eq!(key("write_stdin", r#"{"session_id":123}"#), Some(expected));
        assert_eq!(
            key("wait", r#"{"cell_id":"123"}"#),
            handle_key("cell", "123")
        );
    }
    #[test]
    fn rejects_unexecuted_dynamic_ambiguous_or_interactive_polls() {
        for source in [
            "text('await tools.write_stdin({session_id:123})')",
            "// await tools.write_stdin({session_id:123})",
            "if(false) await tools.write_stdin({session_id:123});",
            "const f = async () => await tools.write_stdin({session_id:123});",
            "text(await tools.write_stdin({session_id:id}));",
            "text(await tools.write_stdin({...other,session_id:123}));",
            "text(await tools.write_stdin({session_id:123,session_id:456}));",
            "text(await tools.write_stdin({session_id:123,chars:'yes'}));",
            "text(await tools.write_stdin({session_id:123})); text(await tools.write_stdin({session_id:456}));",
            "const text = await tools.write_stdin({session_id:123}); text(text);",
        ] {
            assert!(key("exec", source).is_none(), "accepted nonliteral poll");
        }
        assert!(key("wait", r#"{"cell_id":"123","terminate":true}"#).is_none());
        assert!(key("exec", &format!("{}write_stdin", "!".repeat(10_000))).is_none());
    }
}
