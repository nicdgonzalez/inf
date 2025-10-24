from typing import Any

from .parser import Parser
from .section import Section


class Document:
    def __init__(self, sections: list[Section]) -> None:
        self.sections = sections

    @classmethod
    def parse(cls, p: Parser) -> "Document":
        sections: list[Section] = []

        while p.current is not None:
            sections.append(Section.parse(p))

        return cls(sections=sections)

    def as_dict(self) -> dict[str, Any]:
        # INF specification states:
        #
        # > Section names, entries, and directives are case-insensitive.
        #
        # <https://learn.microsoft.com/en-us/windows-hardware/drivers/install/general-syntax-rules-for-inf-files#-case-sensitivity>
        d: dict[str, Any] = CaseInsensitiveDict()

        for section in self.sections:
            d[section.name] = CaseInsensitiveDict(
                {
                    # Each element represents a single line in the file.
                    "": [
                        # Flatten the inner lists if there is only one element.
                        e.value[0] if len(e.value) == 1 else e.value
                        for e in section.entries
                        if e.key is None
                    ],
                    **{
                        e.key: e.value
                        for e in section.entries
                        if e.key is not None
                    },
                }
            )

        return d


class CaseInsensitiveKey:
    def __init__(self, key: str) -> None:
        self.key = key

    def __hash__(self) -> int:
        return hash(self.key.lower())

    def __eq__(self, other: object) -> bool:
        match other:
            case CaseInsensitiveKey():
                return self.key.lower() == other.key.lower()
            case str():
                return self.key.lower() == other.lower()
            case _:
                return NotImplemented

    def __str__(self) -> str:
        return self.key

    def __repr__(self) -> str:
        return repr(self.key)


class CaseInsensitiveDict(dict[str, Any]):
    def __setitem__(self, key: str, value: Any) -> None:
        super().__setitem__(CaseInsensitiveKey(key), value)

    def __getitem__(self, key: str) -> Any:
        return super().__getitem__(CaseInsensitiveKey(key))
