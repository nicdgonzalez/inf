import string

from .token import Token, TokenKind


class Lexer:
    def __init__(self, input: str) -> None:
        self.input = input

    def __iter__(self) -> "LexerIterator":
        return LexerIterator(self.input)


class LexerIterator:
    # Newlines are significant in this file format.
    _WHITESPACE = string.whitespace.replace("\n", "")

    def __init__(self, input: str) -> None:
        self.input = input
        self.index = 0

    def __iter__(self) -> "LexerIterator":
        return self

    def __next__(self) -> Token:
        while self.is_whitespace():
            self.index += 1

        try:
            char = self.input[self.index]
        except IndexError:
            raise StopIteration
        else:
            self.index += 1

        match char:
            case "\n":
                return Token(literal=char, kind=TokenKind.NEWLINE)
            case "=":
                return Token(literal=char, kind=TokenKind.ASSIGN)
            case ",":
                return Token(literal=char, kind=TokenKind.COMMA)
            case "[":
                return Token(literal=char, kind=TokenKind.LBRACKET)
            case "]":
                return Token(literal=char, kind=TokenKind.RBRACKET)
            case ";":
                return self.handle_comments()
            case '"':
                return self.handle_string()
            case c if c in string.ascii_letters:
                return self.handle_identifier(c)
            case d if d in string.digits:
                return self.handle_integer(d)
            case _:
                return Token(literal=char, kind=TokenKind.ILLEGAL)

    def is_whitespace(self) -> bool:
        return (
            self.index < len(self.input)
            and self.input[self.index] in self._WHITESPACE
        )

    def handle_comments(self) -> Token:
        while self.index < len(self.input) and self.input[self.index] != "\n":
            self.index += 1
        else:
            self.index += 1  # Also skip the newline.

        return next(self)

    def handle_string(self) -> Token:
        literal = ""

        while self.index < len(self.input):
            c = self.input[self.index]
            self.index += 1

            if c == '"':
                break

            literal += c

        return Token(literal=literal, kind=TokenKind.STRING)

    def handle_identifier(self, char: str) -> Token:
        literal = char

        while self.index < len(self.input) and (
            (c := self.input[self.index]) in string.ascii_letters
            or c.isnumeric()
            or c in "._-"
        ):
            literal += c
            self.index += 1

        return Token(literal=literal, kind=TokenKind.IDENTIFIER)

    def handle_integer(self, char: str) -> Token:
        literal = char

        while (
            self.index < len(self.input)
            and (c := self.input[self.index]).isdigit()
        ):
            literal += c
            self.index += 1

        return Token(literal=literal, kind=TokenKind.INTEGER)
