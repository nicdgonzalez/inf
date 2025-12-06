import logging
from typing import Iterator

from .token import Token

logger = logging.getLogger(__name__)


class Parser:
    def __init__(self, tokens: Iterator[Token]) -> None:
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
