from typing import Any

from .document import CaseInsensitiveDict, CaseInsensitiveKey


def expand_vars(
    value: str,
    /,
    strings: CaseInsensitiveDict[CaseInsensitiveKey, Any],
) -> str:
    start, end = 0, 0

    while start < len(value):
        try:
            start = value.index("%", start) + 1
            end = value.index("%", start)
        except ValueError:
            break

        var = value[start:end]
        value = value.replace(f"%{var}%", strings[var][0])
        start = end + 1

    return value
