import unittest

import inf

INSTALL_INF = r"""[Version]
signature="$CHICAGO$"

[DefaultInstall]
; These appear to be commands to execute.
CopyFiles = Scheme.Cur, Scheme.Txt
AddReg    = Scheme.Reg

[DestinationDirs]
; 10 is a Windows Directory ID meaning %SystemRoot% (e.g., `C:\Windows`).
Scheme.Cur = 10,"%CUR_DIR%"
Scheme.Txt = 10,"%CUR_DIR%"

[Scheme.Reg]
; Add to Windows registry:
; Arrow, Help, AppStarting, Wait, Crosshair, IBeam, NWPen, No, SizeNS, SizeWE, SizeNWSE, SizeNESW, SizeAll, UpArrow, Hand
; https://thebitguru.com/articles/programmatically-changing-windows-mouse-cursors/3
HKCU,"Control Panel\Cursors\Schemes","%SCHEME_NAME%",,"%10%\%CUR_DIR%\%pointer%,%10%\%CUR_DIR%\%help%,%10%\%CUR_DIR%\%work%,%10%\%CUR_DIR%\%busy%,%10%\%CUR_DIR%\%cross%,%10%\%CUR_DIR%\%Text%,%10%\%CUR_DIR%\%Hand%,%10%\%CUR_DIR%\%unavailiable%,%10%\%CUR_DIR%\%Vert%,%10%\%CUR_DIR%\%Horz%,%10%\%CUR_DIR%\%Dgn1%,%10%\%CUR_DIR%\%Dgn2%,%10%\%CUR_DIR%\%move%,%10%\%CUR_DIR%\%alternate%,%10%\%CUR_DIR%\%link%"

[Scheme.Cur]
; Copy these cursor files to directory in `DestinationDirs`.
Suisei normal.ani
Suisei help.ani
Suisei work.ani
Suisei busy.ani
Suisei text.ani
Suisei unavailable.ani
Suisei vert.ani
Suisei horz.ani
Suisei dgn1.ani
Suisei dgn2.ani
Suisei move.ani
Suisei link.ani
Suisei precision.ani
Suisei hand.ani
Suisei alt.ani

[Scheme.Txt]
; No text files need to be copied.

[Strings]
CUR_DIR       = "Cursors\Hoshimachi Suisei Cursor"
SCHEME_NAME   = "Hoshimachi Suisei Cursor"
pointer       = "Suisei normal.ani"
help          = "Suisei help.ani"
work          = "Suisei work.ani"
busy          = "Suisei busy.ani"
text          = "Suisei text.ani"
unavailiable  = "Suisei unavailable.ani"
vert          = "Suisei vert.ani"
horz          = "Suisei horz.ani"
dgn1          = "Suisei dgn1.ani"
dgn2          = "Suisei dgn2.ani"
move          = "Suisei move.ani"
link          = "Suisei link.ani"
cross         = "Suisei precision.ani"
hand          = "Suisei hand.ani"
alternate     = "Suisei alt.ani"
"""


class TestLoad(unittest.TestCase):
    def test_load(self) -> None:
        data = inf.load(INSTALL_INF)

        self.assertEqual(
            tuple(data.keys()),
            (
                "Version",
                "DefaultInstall",
                "DestinationDirs",
                "Scheme.Reg",
                "Scheme.Cur",
                "Scheme.Txt",
                "Strings",
            ),
        )
