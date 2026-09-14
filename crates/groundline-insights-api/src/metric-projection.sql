toUInt8(
    root_status != 'PASS'
    OR (delegated_status != 'PASS'
        AND (delegated_count > 0 OR delegated_status != 'INSUFFICIENT_EVIDENCE'))
    OR (guardian_status != 'PASS'
        AND (guardian_count > 0 OR guardian_review_count > 0
             OR guardian_incomplete_excluded_count > 0
             OR guardian_status != 'INSUFFICIENT_EVIDENCE'))
) AS component_nonpass,
if(JSONHas(payload_json, 'analysis'), JSONExtractString(payload_json, 'analysis', 'purpose'), 'unclassified') AS purpose
