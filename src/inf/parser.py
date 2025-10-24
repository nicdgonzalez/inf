from typing import Iterable

from .token import Token


class Parser:
    def __init__(self, tokens: Iterable[Token]) -> None:
        self._tokens = tokens
        self._current: Token | None = None
        self._next: Token | None = None

        self.advance()
        self.advance()

    @property
    def current(self) -> Token | None:
        return self._current

    @property
    def next(self) -> Token | None:
        return self._next

    def advance(self) -> None:
        self._current = self._next

        try:
            self._next = next(self._tokens)
        except StopIteration:
            self._next = None
