from typing import Any

from .document import Document
from .lexer import Lexer
from .parser import Parser

__all__ = ("load",)


def load(data: str) -> dict[str, Any]:
    """Deserialize INF-encoded `data`.

    INF is a text-based Setup Information format for Windows-based software
    and drivers.

    <https://learn.microsoft.com/en-us/windows-hardware/drivers/install/general-syntax-rules-for-inf-files>

    Parameters
    ----------
    data
        Text to deserialize.

    Returns
    -------
    dict
        Data deserialized into a Python `dict` with sections as keys,
        and entries as values.

    Warnings
    --------
    I implemented only what I needed without referencing the formal
    specification. The parsing is nowhere near accurate. Proceed with caution!
    """
    lexer = Lexer(input=data)
    tokens = iter(lexer)
    parser = Parser(tokens)
    document = Document.parse(parser)
    return document.as_dict()


del Any
