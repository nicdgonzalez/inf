import dataclasses
import enum


class TokenKind(enum.IntEnum):
    ILLEGAL = enum.auto()

    IDENTIFIER = enum.auto()
    STRING = enum.auto()
    INTEGER = enum.auto()

    NEWLINE = enum.auto()
    ASSIGN = enum.auto()
    COMMA = enum.auto()
    LBRACKET = enum.auto()
    RBRACKET = enum.auto()


@dataclasses.dataclass
class Token:
    literal: str
    kind: TokenKind
