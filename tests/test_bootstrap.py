from __future__ import annotations

import oxirinth
from oxirinth import _core


def test_core_bootstrap():
    assert oxirinth.__version__ == "0.1.0"
    assert _core.__version__ == "0.1.0"
    assert _core.engine_version() == "0.1.0"
