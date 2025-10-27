from .parser import Parser
from .token import TokenKind


class Entry:
    """Represents a key/value pair (or value only) of a `Section`."""

    def __init__(self, key: str | None, value: list[str | int]) -> None:
        self.key = key
        self.value = value

    @classmethod
    def parse(cls, p: Parser, /) -> "Entry":
        assert p.current is not None, "called parse after end of input"
        assert p.current.kind == TokenKind.IDENTIFIER, p.current.kind.name

        key = p.current.literal
        p.advance()

        while p.current is not None and p.current.kind in (
            TokenKind.IDENTIFIER,
            TokenKind.INTEGER,
        ):
            if p.current.kind == TokenKind.IDENTIFIER:
                # It wasn't counted as a single identifier because of a space;
                # now we know it's part of the same identifier so we can just
                # add it back in.
                key += " "

            key += p.current.literal
            p.advance()

        if p.current is None:
            # Nothing left to parse; we are left with only an identifier,
            # so we'll pretend it was a string and return it as a value.
            return cls(key=None, value=[key])

        match p.current.kind:
            case TokenKind.NEWLINE:
                p.advance()
                # Same as before -- it has no value, so it becomes the value.
                return cls(key=None, value=[key])
            case TokenKind.ASSIGN:
                p.advance()

                if p.current is None:
                    raise RuntimeError("unexpected end of input")

                value: list[str | int] = []

                while (
                    p.current is not None
                    and p.current.kind != TokenKind.NEWLINE
                ):
                    if p.current.kind == TokenKind.INTEGER:
                        assert p.current.literal.isnumeric(), p.current.literal
                        i = int(p.current.literal)
                        value.append(i)
                    elif p.current.kind == TokenKind.COMMA:
                        # The value is still significant, but it's empty.
                        value.append("")
                    else:
                        value.append(p.current.literal)

                    p.advance()

                return cls(key=key, value=value)
            case TokenKind.COMMA:
                # The entry has no key; the first identifier was the first
                # element of an array. I suspect `IDENTIFIER` is not the only
                # valid token kind for the first slot of an entry, but I have
                # yet to see otherwise, so it will stay like this for now.
                value = [key]
                p.advance()

                while (
                    p.current is not None
                    and p.current.kind != TokenKind.NEWLINE
                ):
                    if p.current.kind == TokenKind.INTEGER:
                        assert p.current.literal.isnumeric(), p.current.literal
                        i = int(p.current.literal)
                        value.append(i)
                        p.advance()
                    elif p.current.kind == TokenKind.COMMA:
                        # The value is still significant, but it's empty.
                        value.append("")
                        # There is no item, so no need to advance.
                    else:
                        value.append(p.current.literal)
                        p.advance()

                    # Check before advancing to avoid advancing passed
                    # the newline on the last element of the array.
                    if p.current.kind == TokenKind.COMMA:
                        p.advance()

                return cls(key=None, value=value)
            case _:
                raise RuntimeError(f"unexpected token: {p.current!r}")
