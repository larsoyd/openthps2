# Progress

THawk2.exe holds 4720 code functions (959,739 bytes). Each function is attributed to its original source file through the assert strings the executable still carries; functions without one take the file of their neighbours when both sides agree.

`implemented` means the routine is reimplemented in Rust and checked frame by frame against an independent model of the recovered behaviour. `recovered` means the behaviour has been read from the binary and documented but not yet reimplemented. Everything else is open.

## Overall

| | Functions | Bytes |
|---|---|---|
| Implemented | 89 (1.9%) | 65,298 (6.8%) |
| Recovered | 92 (1.9%) | 45,154 (4.7%) |
| Open | 4539 | 849,287 |

`█░░░░░░░░░░░░░░░░░░░` 6.8% implemented by bytes

## By original source file

| Source file | Functions | Bytes | Implemented | Recovered | Progress |
|---|---|---|---|---|---|
| unattributed | 2977 | 354,046 | 34 | 35 | `░░░░░░░░░░` 4% |
| physics.cpp | 110 | 64,313 | 29 | 44 | `█████░░░░░` 51% |
| mmu.cpp | 70 | 31,238 | 0 | 0 | `░░░░░░░░░░` 0% |
| front.cpp | 42 | 29,072 | 0 | 0 | `░░░░░░░░░░` 0% |
| bit.cpp | 123 | 25,393 | 0 | 0 | `░░░░░░░░░░` 0% |
| LevelEditor.cpp | 56 | 23,929 | 0 | 0 | `░░░░░░░░░░` 0% |
| LevelGen.cpp | 38 | 23,037 | 0 | 0 | `░░░░░░░░░░` 0% |
| options.cpp | 54 | 22,299 | 0 | 1 | `░░░░░░░░░░` 0% |
| create.cpp | 54 | 18,174 | 0 | 0 | `░░░░░░░░░░` 0% |
| GenObjs.cpp | 58 | 15,497 | 0 | 0 | `░░░░░░░░░░` 0% |
| panel.cpp | 37 | 14,181 | 0 | 4 | `░░░░░░░░░░` 0% |
| career.cpp | 81 | 13,988 | 0 | 2 | `░░░░░░░░░░` 0% |
| trig.cpp | 30 | 13,716 | 1 | 2 | `░░░░░░░░░░` 2% |
| main.cpp | 29 | 13,255 | 1 | 0 | `░░░░░░░░░░` 3% |
| spool.cpp | 41 | 12,775 | 6 | 0 | `████░░░░░░` 36% |
| Menu.cpp | 51 | 10,954 | 0 | 0 | `░░░░░░░░░░` 0% |
| competition.cpp | 30 | 10,871 | 0 | 0 | `░░░░░░░░░░` 0% |
| playsel.cpp | 37 | 10,008 | 0 | 0 | `░░░░░░░░░░` 0% |
| video.cpp | 28 | 10,005 | 0 | 0 | `░░░░░░░░░░` 0% |
| d3dtex.cpp | 20 | 9,174 | 7 | 0 | `████░░░░░░` 41% |
| EquipSel.cpp | 44 | 8,492 | 0 | 0 | `░░░░░░░░░░` 0% |
| trickwin.cpp | 17 | 8,388 | 0 | 0 | `░░░░░░░░░░` 0% |
| camera.cpp | 22 | 8,285 | 0 | 0 | `░░░░░░░░░░` 0% |
| m3d.cpp | 16 | 7,875 | 5 | 0 | `██████░░░░` 64% |
| camera.h | 1 | 7,297 | 0 | 0 | `░░░░░░░░░░` 0% |
| NetGame.cpp | 23 | 6,946 | 0 | 0 | `░░░░░░░░░░` 0% |
| cresel.cpp | 8 | 6,893 | 0 | 0 | `░░░░░░░░░░` 0% |
| baddy.cpp | 10 | 6,545 | 0 | 0 | `░░░░░░░░░░` 0% |
| trickdb.cpp | 22 | 6,398 | 0 | 0 | `░░░░░░░░░░` 0% |
| FrontEnd2.cpp | 10 | 6,381 | 0 | 0 | `░░░░░░░░░░` 0% |
| WinMain.cpp | 13 | 5,956 | 0 | 0 | `░░░░░░░░░░` 0% |
| appwin.cpp | 12 | 5,751 | 0 | 0 | `░░░░░░░░░░` 0% |
| m3dcolij.cpp | 6 | 5,728 | 1 | 0 | `██░░░░░░░░` 20% |
| cre_texset.cpp | 4 | 5,005 | 0 | 0 | `░░░░░░░░░░` 0% |
| PCMovie.cpp | 7 | 4,977 | 0 | 0 | `░░░░░░░░░░` 0% |
| trickscript.cpp | 2 | 4,585 | 0 | 0 | `░░░░░░░░░░` 0% |
| create_stitch.cpp | 15 | 4,319 | 0 | 0 | `░░░░░░░░░░` 0% |
| ob.cpp | 29 | 4,297 | 2 | 3 | `██░░░░░░░░` 24% |
| soundlib.cpp | 14 | 4,264 | 0 | 0 | `░░░░░░░░░░` 0% |
| Image.cpp | 32 | 4,213 | 0 | 0 | `░░░░░░░░░░` 0% |
| powerup.cpp | 14 | 4,208 | 0 | 0 | `░░░░░░░░░░` 0% |
| statwin.cpp | 13 | 4,153 | 0 | 0 | `░░░░░░░░░░` 0% |
| FontTools.cpp | 20 | 4,086 | 0 | 0 | `░░░░░░░░░░` 0% |
| dust.cpp | 7 | 3,770 | 0 | 0 | `░░░░░░░░░░` 0% |
| commonfe.cpp | 26 | 3,710 | 0 | 0 | `░░░░░░░░░░` 0% |
| rail.cpp | 14 | 3,610 | 0 | 0 | `░░░░░░░░░░` 0% |
| CreEdit.cpp | 9 | 3,519 | 0 | 0 | `░░░░░░░░░░` 0% |
| platform.cpp | 4 | 3,463 | 0 | 0 | `░░░░░░░░░░` 0% |
| skatmgr.cpp | 30 | 3,436 | 0 | 0 | `░░░░░░░░░░` 0% |
| DxPlay.cpp | 24 | 2,986 | 0 | 0 | `░░░░░░░░░░` 0% |
| decomp.cpp | 2 | 2,930 | 0 | 0 | `░░░░░░░░░░` 0% |
| mem.cpp | 11 | 2,904 | 0 | 0 | `░░░░░░░░░░` 0% |
| player_spool.cpp | 11 | 2,885 | 0 | 0 | `░░░░░░░░░░` 0% |
| statedit.cpp | 7 | 2,740 | 0 | 0 | `░░░░░░░░░░` 0% |
| fileio.cpp | 11 | 2,547 | 0 | 0 | `░░░░░░░░░░` 0% |
| EditorGlobal.cpp | 14 | 2,379 | 0 | 0 | `░░░░░░░░░░` 0% |
| m3dzone.cpp | 2 | 2,370 | 1 | 0 | `████████░░` 77% |
| Credits.cpp | 7 | 2,201 | 0 | 0 | `░░░░░░░░░░` 0% |
| mech.cpp | 9 | 2,197 | 0 | 1 | `░░░░░░░░░░` 0% |
| Spooler.cpp | 17 | 2,191 | 0 | 0 | `░░░░░░░░░░` 0% |
| d3dpoly.cpp | 7 | 2,138 | 0 | 0 | `░░░░░░░░░░` 0% |
| create_color.cpp | 5 | 2,129 | 0 | 0 | `░░░░░░░░░░` 0% |
| pal.cpp | 12 | 2,059 | 1 | 0 | `████░░░░░░` 36% |
| m3dutils.cpp | 4 | 1,997 | 0 | 0 | `░░░░░░░░░░` 0% |
| FaceRemoval.cpp | 3 | 1,841 | 0 | 0 | `░░░░░░░░░░` 0% |
| Shatter.cpp | 1 | 1,689 | 0 | 0 | `░░░░░░░░░░` 0% |
| tricksel.cpp | 3 | 1,634 | 0 | 0 | `░░░░░░░░░░` 0% |
| screentools.cpp | 10 | 1,625 | 0 | 0 | `░░░░░░░░░░` 0% |
| Mess.cpp | 13 | 1,614 | 0 | 0 | `░░░░░░░░░░` 0% |
| d3dfunc.cpp | 7 | 1,614 | 0 | 0 | `░░░░░░░░░░` 0% |
| player_ui.cpp | 5 | 1,590 | 0 | 0 | `░░░░░░░░░░` 0% |
| redbook.cpp | 10 | 1,462 | 0 | 0 | `░░░░░░░░░░` 0% |
| trickob.cpp | 6 | 1,419 | 0 | 0 | `░░░░░░░░░░` 0% |
| texteffects.cpp | 17 | 1,332 | 0 | 0 | `░░░░░░░░░░` 0% |
| create_ui.cpp | 6 | 1,325 | 0 | 0 | `░░░░░░░░░░` 0% |
| PRE.cpp | 9 | 1,246 | 0 | 0 | `░░░░░░░░░░` 0% |
| d3dsprite.cpp | 2 | 1,243 | 0 | 0 | `░░░░░░░░░░` 0% |
| bit2.cpp | 8 | 1,144 | 0 | 0 | `░░░░░░░░░░` 0% |
| m3dinit.cpp | 3 | 1,138 | 1 | 0 | `██████░░░░` 62% |
| db.cpp | 3 | 1,116 | 0 | 0 | `░░░░░░░░░░` 0% |
| DynamicLogo.cpp | 3 | 1,088 | 0 | 0 | `░░░░░░░░░░` 0% |
| pad.cpp | 8 | 1,043 | 0 | 0 | `░░░░░░░░░░` 0% |
| carsel.cpp | 5 | 1,032 | 0 | 0 | `░░░░░░░░░░` 0% |
| perwin.cpp | 4 | 995 | 0 | 0 | `░░░░░░░░░░` 0% |
| goalwin.cpp | 1 | 979 | 0 | 0 | `░░░░░░░░░░` 0% |
| PcInput.cpp | 1 | 970 | 0 | 0 | `░░░░░░░░░░` 0% |
| tapes.cpp | 1 | 803 | 0 | 0 | `░░░░░░░░░░` 0% |
| custmgr.cpp | 2 | 765 | 0 | 0 | `░░░░░░░░░░` 0% |
| videowin.cpp | 1 | 756 | 0 | 0 | `░░░░░░░░░░` 0% |
| EditorPieces2.cpp | 6 | 737 | 0 | 0 | `░░░░░░░░░░` 0% |
| medals.cpp | 1 | 660 | 0 | 0 | `░░░░░░░░░░` 0% |
| LevelSel.cpp | 1 | 646 | 0 | 0 | `░░░░░░░░░░` 0% |
| cre_tex.cpp | 2 | 645 | 0 | 0 | `░░░░░░░░░░` 0% |
| init.cpp | 1 | 600 | 0 | 0 | `░░░░░░░░░░` 0% |
| car.cpp | 1 | 570 | 0 | 0 | `░░░░░░░░░░` 0% |
| MainMenu.cpp | 2 | 511 | 0 | 0 | `░░░░░░░░░░` 0% |
| tricktips.cpp | 2 | 499 | 0 | 0 | `░░░░░░░░░░` 0% |
| sfx.cpp | 3 | 493 | 0 | 0 | `░░░░░░░░░░` 0% |
| D3dTimer.cpp | 2 | 424 | 0 | 0 | `░░░░░░░░░░` 0% |
| NetMenu.cpp | 1 | 341 | 0 | 0 | `░░░░░░░░░░` 0% |
| cresave.cpp | 1 | 333 | 0 | 0 | `░░░░░░░░░░` 0% |
| vidmenu.cpp | 1 | 319 | 0 | 0 | `░░░░░░░░░░` 0% |
| thstub.cpp | 1 | 312 | 0 | 0 | `░░░░░░░░░░` 0% |
| multisel.cpp | 1 | 301 | 0 | 0 | `░░░░░░░░░░` 0% |
| exp.cpp | 1 | 289 | 0 | 0 | `░░░░░░░░░░` 0% |
| gamefmv.cpp | 1 | 264 | 0 | 0 | `░░░░░░░░░░` 0% |
| create_spool.cpp | 1 | 226 | 0 | 0 | `░░░░░░░░░░` 0% |
| handicap.cpp | 1 | 208 | 0 | 0 | `░░░░░░░░░░` 0% |
| textwin.cpp | 1 | 197 | 0 | 0 | `░░░░░░░░░░` 0% |
| tag.cpp | 1 | 170 | 0 | 0 | `░░░░░░░░░░` 0% |
| backgrnd.cpp | 1 | 167 | 0 | 0 | `░░░░░░░░░░` 0% |
| campt.cpp | 1 | 130 | 0 | 0 | `░░░░░░░░░░` 0% |
| Too many PSX files loaded, increase MAXPSXS in spool.h | 1 | 106 | 0 | 0 | `░░░░░░░░░░` 0% |

## Function map

Implemented routines and the Rust module that holds them.

| Address | Size | Source file | Rust module |
|---|---|---|---|
| 0045f530 | 4024 | unattributed | projection.rs |
| 004604f0 | 1429 | m3d.cpp | draw.rs |
| 00460ca0 | 1092 | m3d.cpp | draw.rs |
| 004610f0 | 2388 | m3d.cpp | draw.rs |
| 00461a90 | 50 | m3d.cpp | projection.rs |
| 00461b70 | 50 | m3d.cpp | projection.rs |
| 004624d0 | 1046 | unattributed | collision.rs |
| 004638d0 | 1143 | m3dcolij.cpp | collision.rs |
| 004647c0 | 704 | m3dinit.cpp | draw.rs |
| 00465f60 | 201 | unattributed | skater.rs |
| 00466090 | 21 | unattributed | collision.rs |
| 004660b0 | 1827 | m3dzone.cpp | collision.rs |
| 00468b30 | 444 | main.cpp | ground.rs |
| 0046c670 | 167 | unattributed | ground.rs |
| 0046d970 | 252 | unattributed | ground.rs |
| 00480730 | 340 | ob.cpp | anim.rs |
| 00480950 | 709 | ob.cpp | anim.rs |
| 0048a7e0 | 749 | pal.cpp | materials.rs |
| 0048ea80 | 131 | unattributed | collision.rs |
| 0048f3a0 | 491 | physics.cpp | ground.rs |
| 004900b0 | 549 | physics.cpp | ground.rs |
| 004903f0 | 40 | physics.cpp | anim.rs |
| 00490420 | 43 | physics.cpp | anim.rs |
| 00490450 | 46 | physics.cpp | anim.rs |
| 00490480 | 49 | physics.cpp | anim.rs |
| 00490610 | 110 | physics.cpp | ground.rs |
| 00490680 | 169 | physics.cpp | ground.rs |
| 00491780 | 1010 | physics.cpp | ground.rs |
| 00492ed0 | 70 | physics.cpp | ground.rs |
| 00492f20 | 1080 | physics.cpp | ground.rs |
| 00493370 | 2707 | physics.cpp | ground.rs |
| 004956f0 | 197 | physics.cpp | ground.rs |
| 004957c0 | 802 | physics.cpp | ground.rs |
| 00496060 | 534 | physics.cpp | skater.rs |
| 00496360 | 491 | physics.cpp | ground.rs |
| 00496550 | 5117 | physics.cpp | ground.rs |
| 00497960 | 306 | physics.cpp | ground.rs |
| 00497f40 | 5285 | physics.cpp | ground.rs |
| 0049a280 | 3240 | physics.cpp | ground.rs |
| 0049b500 | 1477 | physics.cpp | ground.rs |
| 0049bad0 | 1421 | physics.cpp | ground.rs |
| 0049c060 | 716 | physics.cpp | ground.rs |
| 0049c330 | 1172 | physics.cpp | ground.rs |
| 0049d080 | 612 | physics.cpp | ground.rs |
| 0049d480 | 690 | physics.cpp | ground.rs |
| 0049d9c0 | 271 | physics.cpp | ground.rs |
| 0049db80 | 826 | physics.cpp | ground.rs |
| 0049e680 | 2983 | physics.cpp | ground.rs |
| 004b20f0 | 33 | spool.cpp | psx.rs |
| 004b2450 | 1578 | spool.cpp | anim.rs |
| 004b2b70 | 1020 | spool.cpp | bitmap.rs |
| 004b39b0 | 1088 | spool.cpp | materials.rs |
| 004b43e0 | 602 | spool.cpp | materials.rs |
| 004b46a0 | 247 | spool.cpp | materials.rs |
| 004c4d10 | 286 | trig.cpp | ground.rs |
| 004c9f50 | 95 | unattributed | trig.rs |
| 004ca9f0 | 41 | unattributed | skater.rs |
| 004caa20 | 41 | unattributed | skater.rs |
| 004cabf0 | 49 | unattributed | skater.rs |
| 004cac30 | 46 | unattributed | skater.rs |
| 004cad30 | 35 | unattributed | skater.rs |
| 004d0a20 | 22 | unattributed | materials.rs |
| 004d0a40 | 27 | unattributed | materials.rs |
| 004d14d0 | 988 | unattributed | draw.rs |
| 004d18b0 | 175 | unattributed | draw.rs |
| 004d1d40 | 936 | unattributed | draw.rs |
| 004d20f0 | 530 | unattributed | draw.rs |
| 004d29e0 | 959 | unattributed | draw.rs |
| 004d8cd0 | 285 | d3dtex.cpp | bitmap.rs |
| 004d8df0 | 273 | d3dtex.cpp | materials.rs |
| 004d8f10 | 1010 | d3dtex.cpp | bitmap.rs |
| 004d9310 | 794 | d3dtex.cpp | materials.rs |
| 004d9630 | 592 | d3dtex.cpp | bitmap.rs |
| 004d9f50 | 770 | d3dtex.cpp | materials.rs |
| 004da3e0 | 23 | d3dtex.cpp | materials.rs |
| 004da680 | 242 | unattributed | bitmap.rs |
| 004da840 | 666 | unattributed | materials.rs |
| 004e2ff0 | 113 | unattributed | skater.rs |
| 004e3130 | 160 | unattributed | skater.rs |
| 004e7c60 | 374 | unattributed | skater.rs |
| 004e7de0 | 377 | unattributed | skater.rs |
| 004e7f60 | 371 | unattributed | skater.rs |
| 004e80e0 | 85 | unattributed | skater.rs |
| 004e8770 | 3 | unattributed | bitmap.rs |
| 004e9780 | 421 | unattributed | pkr.rs |
| 004e9940 | 568 | unattributed | pkr.rs |
| 004f53b0 | 44 | unattributed | skater.rs |
| 004f5f90 | 39 | unattributed | skater.rs |
| 004f5fc0 | 19 | unattributed | skater.rs |

## Recovered, not yet reimplemented

| Address | Size | Source file |
|---|---|---|
| 00413c10 | 325 | career.cpp |
| 00413d60 | 357 | career.cpp |
| 00464200 | 1004 | unattributed |
| 0046c720 | 1212 | mech.cpp |
| 0046d6e0 | 58 | unattributed |
| 0046e080 | 333 | unattributed |
| 0046e1d0 | 106 | unattributed |
| 00480890 | 192 | ob.cpp |
| 00480fa0 | 41 | ob.cpp |
| 00480ff0 | 273 | ob.cpp |
| 00487490 | 1911 | options.cpp |
| 00489930 | 88 | unattributed |
| 00489990 | 127 | unattributed |
| 00489a10 | 674 | unattributed |
| 0048cbc0 | 374 | panel.cpp |
| 0048cd40 | 59 | panel.cpp |
| 0048d6d0 | 58 | panel.cpp |
| 0048ddc0 | 1346 | panel.cpp |
| 0048f5f0 | 129 | physics.cpp |
| 0048f7c0 | 56 | physics.cpp |
| 0048f8a0 | 67 | physics.cpp |
| 0048f8f0 | 548 | physics.cpp |
| 0048fb20 | 189 | physics.cpp |
| 0048ffc0 | 71 | physics.cpp |
| 004902e0 | 267 | physics.cpp |
| 004904c0 | 11 | physics.cpp |
| 004904d0 | 311 | physics.cpp |
| 00490730 | 932 | physics.cpp |
| 00490ef0 | 1482 | physics.cpp |
| 004914d0 | 502 | physics.cpp |
| 004916d0 | 172 | physics.cpp |
| 00491b80 | 129 | physics.cpp |
| 00491c90 | 270 | physics.cpp |
| 00492030 | 234 | physics.cpp |
| 00492120 | 99 | physics.cpp |
| 00492190 | 255 | physics.cpp |
| 00492400 | 233 | physics.cpp |
| 00492560 | 118 | physics.cpp |
| 004925e0 | 1195 | physics.cpp |
| 00492ea0 | 41 | physics.cpp |
| 00493e10 | 289 | physics.cpp |
| 00494090 | 213 | physics.cpp |
| 00494210 | 5330 | physics.cpp |
| 00495af0 | 457 | physics.cpp |
| 00495cc0 | 927 | physics.cpp |
| 00496280 | 101 | physics.cpp |
| 00496300 | 96 | physics.cpp |
| 00497aa0 | 272 | physics.cpp |
| 00497bb0 | 568 | physics.cpp |
| 00497df0 | 321 | physics.cpp |
| 004995d0 | 305 | physics.cpp |
| 00499710 | 2710 | physics.cpp |
| 0049a1b0 | 41 | physics.cpp |
| 0049a1e0 | 98 | physics.cpp |
| 0049b010 | 1224 | physics.cpp |
| 0049c7d0 | 120 | physics.cpp |
| 0049c850 | 127 | physics.cpp |
| 0049d740 | 329 | physics.cpp |
| 0049d8a0 | 283 | physics.cpp |
| 0049dad0 | 168 | physics.cpp |
| 0049df00 | 1296 | physics.cpp |
| 0049e570 | 268 | physics.cpp |
| 004a12d0 | 354 | unattributed |
| 004aaf70 | 2622 | unattributed |
| 004bb240 | 132 | unattributed |
| 004bb2d0 | 262 | unattributed |
| 004c4e30 | 715 | trig.cpp |
| 004c5dc0 | 6201 | trig.cpp |
| 004c9340 | 77 | unattributed |
| 004c9390 | 107 | unattributed |
| 004ca0c0 | 120 | unattributed |
| 004ca8f0 | 73 | unattributed |
| 004cde70 | 11 | unattributed |
| 004de6d0 | 188 | unattributed |
| 004e2070 | 182 | unattributed |
| 004e2130 | 168 | unattributed |
| 004e7500 | 162 | unattributed |
| 004e75b0 | 803 | unattributed |
| 004e79f0 | 220 | unattributed |
| 004e7ad0 | 132 | unattributed |
| 004e7b60 | 87 | unattributed |
| 004e8410 | 79 | unattributed |
| 004e8480 | 217 | unattributed |
| 004e9730 | 67 | unattributed |
| 004e9b80 | 164 | unattributed |
| 004f3980 | 42 | unattributed |
| 004f39b0 | 42 | unattributed |
| 004f5f10 | 17 | unattributed |
| 004f6510 | 52 | unattributed |
| 00501740 | 25 | unattributed |
| 00502f74 | 233 | unattributed |
| 0050b7a0 | 208 | unattributed |
