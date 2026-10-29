# Configuration reference

`aifix` uses a JSON configuration file for model providers, task routing, execution limits, scanning, and file-access filters.

## Location and creation

By default the file is `$XDG_CONFIG_HOME/aifix/config.json` when `XDG_CONFIG_HOME` is set and nonempty, otherwise `$HOME/.config/aifix/config.json`. Pass `-c` or `--config` to select another file. On first use, `aifix` writes a default configuration. The schema and default values are defined in [`src/config.rs`](../src/config.rs).

## Top-level fields

| Field | Type | Purpose |
| --- | --- | --- |
| `provider` | string | Name of the selected provider (must exist in `providerlist`) |
| `providerlist` | array of objects | Available model providers and their launch/request settings |
| `taskproviderlist` | array of objects | Per-task provider selection and temperature |
| `max_try_count` | object | Workflow and tool-call failure limits |
| `http_retry` | object | Additional retries and fixed delay for HTTP 504 responses from the LLM endpoint |
| `queue_length_max` | integer | Maximum queue length |
| `queue_length_save` | integer | Queue entries to save (must not exceed `queue_length_max`) |
| `scanendfilter` | array of strings | Filename suffixes selected by the scanner |
| `scanfullfilter` | array of strings | Exact filenames selected by the scanner |
| `writefilter` | array of strings | Configured writable filesystem roots |
| `readfilter` | array of strings | Configured readable filesystem roots |

### Provider objects

Each entry in `providerlist` has these fields:

| Field | Meaning |
| --- | --- |
| `name` | Provider identifier |
| `comment` | Human-readable note |
| `source` | Model source or download URL |
| `endpoint` | Model API endpoint |
| `model` | Model filename or identifier |
| `api_key` | API key (avoid committing credentials) |
| `insecure` | Whether to allow insecure connections |
| `llmbin` | Local model server executable |
| `llmmodeldir` | Directory containing local model files |
| `llmparam` | Arguments passed to the model server |

Entries in `taskproviderlist` use `task`, `provider`, and `temperature`. The `provider` must match an entry in `providerlist`; temperature must be finite.

`max_try_count` contains `max_workflow_fail` and `max_tool_call_fail`.

### HTTP 504 retry policy

`http_retry` contains `count` (number of **additional** requests after the initial attempt) and `delay_seconds` (fixed wait between attempts). The defaults are **3 retries** and **30 seconds**. Set `count` to `0` to disable retries:

```json
{
  "http_retry": {
    "count": 3,
    "delay_seconds": 30
  }
}
```

Older configuration files without `http_retry` receive these defaults automatically. The retry policy applies to LLM calls made through `LlmCall` and retries only HTTP 504 errors recognized by the HTTP client; it does not retry all HTTP or model errors. Each retry resends the request, so a gateway timeout may cause duplicate inference work if the server is still processing the previous request. The current detection uses the HTTP client's error text rather than a structured status code; this is a known limitation.

## Filesystem filters and placeholders

Both `writefilter` and `readfilter` accept path placeholders. They are expanded at configuration load time; the JSON file on disk remains unchanged.

| Placeholder | Value |
| --- | --- |
| `{{home}}` | System `HOME` directory |
| `{{projectdir}}` | Absolute canonical current working directory |
| `{{relprojectdir}}` | Project directory relative to `HOME`, without a leading slash |

For example, with `HOME=/home/me` and current directory `/home/me/project/myproj`:

```json
{
  "writefilter": ["{{projectdir}}"],
  "readfilter": [
    "{{projectdir}}/..",
    "{{home}}/.project/{{relprojectdir}}",
    "/opt"
  ]
}
```

The second read entry expands to `/home/me/.project/project/myproj`. `{{relprojectdir}}` is a **fragment**: expansion never implicitly adds `HOME`. Unknown placeholders are rejected. `{{relprojectdir}}` requires the current project directory to be under `HOME`.

Placeholder expansion is textual: it does not create directories, and the resulting strings are not automatically canonicalized as a whole. The filters described here are separate from the `-f / --pathfilter` command-line option.

## Validation and maintenance

The loader checks for duplicate provider names, missing provider references, nonfinite task temperatures, invalid queue limits, and an unknown selected provider. After loading, it expands filesystem filter placeholders.

When the application adds required configuration fields, existing JSON files may require updating. The optional `http_retry` field uses defaults when absent. Consult `Config::default()` in [`src/config.rs`](../src/config.rs) for authoritative current defaults.

## LLM output token limit (CLI)

The global `aifix -k COUNT` / `--tokens COUNT` switch sets the maximum output tokens requested for LLM calls that use the default limit. The default is `30000`; zero, negative, non-numeric and out-of-range values are rejected. Explicitly limited workflow steps retain their own token limits. This setting does not change the model's input context window or local server configuration. Supply the switch before `-a/--args`.
