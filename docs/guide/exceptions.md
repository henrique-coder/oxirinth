# Exceptions

Every exception raised by Oxirinth inherits from `OxirinthError`, defined inside `oxirinth._core` and re-exported from `oxirinth` and `oxirinth.exceptions`. Unexpected Rust panics across the PyO3 FFI boundary are caught with `catch_unwind` and translated into `OxirinthInternalError` rather than crashing the Python interpreter.

## Hierarchy

```text
OxirinthError(Exception)
├── OxirinthConfigError            # Invalid base_url, User-Agent, api_key, timeout, or filter args
├── OxirinthNetworkError           # DNS, TLS, timeout, or connection reset after retries
├── OxirinthAPIError               # Non-2xx Modrinth API response
│   ├── OxirinthNotFoundError      # HTTP 404
│   ├── OxirinthAuthError          # HTTP 401 / 403
│   └── OxirinthRateLimitError     # HTTP 429 (when max_rate_limit_wait is exceeded)
├── OxirinthTypeMismatchError      # Project exists, but requested domain is absent from project_types
├── OxirinthCacheError             # Unrecoverable cache operation failure
├── OxirinthUnboundModelError      # Lazy helper called on an unbound model
└── OxirinthInternalError          # Caught Rust panic or unexpected serialization invariant error
```

## Structured Attributes

- `OxirinthAPIError` (and `OxirinthNotFoundError`, `OxirinthAuthError`, `OxirinthRateLimitError`):
  - `status_code: int`
  - `error_code: str | None`
  - `description: str`
  - `url: str`
- `OxirinthRateLimitError`: also exposes `retry_after_secs: int`.
- `OxirinthTypeMismatchError`:
  - `expected: str`
  - `actual: tuple[str, ...]`
  - `identifier: str`

## Example Handling

```python
from oxirinth import (
    Oxirinth,
    OxirinthAPIError,
    OxirinthNotFoundError,
    OxirinthTypeMismatchError,
)

with Oxirinth() as client:
    try:
        plugin = client.get_plugin("sodium")
    except OxirinthTypeMismatchError as exc:
        print(f"{exc.identifier} expected {exc.expected}, found {exc.actual}")
    except OxirinthNotFoundError as exc:
        print(f"Missing resource ({exc.status_code}): {exc.description}")
    except OxirinthAPIError as exc:
        print(f"API failure {exc.status_code} at {exc.url}: {exc.description}")
```
