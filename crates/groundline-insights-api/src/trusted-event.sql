schema_version = 5
AND runtime_family IN ('codex_app', 'codex_cli')
AND os_family IN ('macos', 'linux', 'windows')
AND execution_mode IN ('desktop', 'local_headless', 'remote_headless')
AND cached_input_tokens <= input_tokens
AND reasoning_output_tokens <= output_tokens
AND total_tokens >= input_tokens
AND total_tokens - least(total_tokens, input_tokens) >= output_tokens
AND selected_root_count <= eligible_root_count
AND root_count = selected_root_count
AND observed_root_count >= root_count
AND unreadable_root_count = 0
AND root_truncated_count = 0
AND truncated_count = 0
AND originator_unclassified_excluded_root_count = 0
AND verification_tool_calls = toUInt64(verification_success_count) + verification_failure_count + verification_unresolved_count
AND period_start IS NOT NULL AND period_end IS NOT NULL
AND period_start < period_end AND period_end <= generated_at
AND generated_at <= received_at + INTERVAL 5 MINUTE
AND arrayAll(usage ->
    JSONExtractUInt(usage, 'cached_input_tokens') <= JSONExtractUInt(usage, 'input_tokens')
    AND JSONExtractUInt(usage, 'reasoning_output_tokens') <= JSONExtractUInt(usage, 'output_tokens')
    AND JSONExtractUInt(usage, 'total_tokens') >= JSONExtractUInt(usage, 'input_tokens')
    AND JSONExtractUInt(usage, 'total_tokens') - least(JSONExtractUInt(usage, 'total_tokens'), JSONExtractUInt(usage, 'input_tokens')) >= JSONExtractUInt(usage, 'output_tokens')
    AND JSONExtractUInt(usage, 'cumulative_rollout_count') + JSONExtractUInt(usage, 'fallback_rollout_count') = JSONExtractUInt(usage, 'rollout_count_with_usage')
    AND if(JSONExtractString(usage, 'source') IN ('unknown', 'unavailable'),
        JSONExtractUInt(usage, 'rollout_count_with_usage') = 0
        AND JSONExtractUInt(usage, 'input_tokens') = 0
        AND JSONExtractUInt(usage, 'output_tokens') = 0
        AND JSONExtractUInt(usage, 'total_tokens') = 0
        AND JSONExtractUInt(usage, 'cache_write_input_tokens') = 0,
        JSONExtractUInt(usage, 'rollout_count_with_usage') > 0)
    AND if(JSONExtractUInt(usage, 'input_tokens') = 0,
        JSONExtractRaw(usage, 'cached_input_ratio') = 'null',
        ifNull(JSONExtract(usage, 'cached_input_ratio', 'Nullable(Float64)') = round(JSONExtractUInt(usage, 'cached_input_tokens') / JSONExtractUInt(usage, 'input_tokens') * 10000) / 10000, 0)),
    [JSONExtractRaw(payload_json, 'metrics', 'root', 'usage'),
     JSONExtractRaw(payload_json, 'metrics', 'delegated', 'usage'),
     JSONExtractRaw(payload_json, 'metrics', 'guardian', 'usage')])
