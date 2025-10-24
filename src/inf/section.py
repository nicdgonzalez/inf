from .entry import Entry
from .parser import Parser
from .token import TokenKind


class Section:
    def __init__(self, name: str, entries: list[Entry]) -> None:
        self.name = name
        self.entries = entries

    @classmethod
    def parse(cls, p: Parser) -> "Section":
        assert p.current is not None, "called parse after end of input"

        if p.current.kind != TokenKind.LBRACKET:
            raise RuntimeError(f"expected LBRACKET, got {p.current.kind.name}")
        else:
            p.advance()

        name = ""

        while p.current is not None and p.current.kind != TokenKind.RBRACKET:
            name += p.current.literal
            p.advance()
        else:
            p.advance()  # Advance passed the RBRACKET.

        entries: list[Entry] = []

        while p.current is not None and p.current.kind == TokenKind.NEWLINE:
            p.advance()  # Ignore empty lines between the section and entries.

        if p.current is None:
            # End of input -- no entries.
            return cls(name=name, entries=entries)
        elif p.current.kind == TokenKind.LBRACKET:
            # Empty section -- no entries.
            return cls(name=name, entries=entries)
        elif p.current.kind != TokenKind.IDENTIFIER:
            raise RuntimeError(
                f"expected IDENTIFIER, got {p.current.kind.name}"
            )
        else:
            pass

        while p.current is not None and p.current.kind != TokenKind.LBRACKET:
            if p.current.kind == TokenKind.NEWLINE:
                p.advance()
            else:
                entries.append(Entry.parse(p))

        return cls(name=name, entries=entries)
