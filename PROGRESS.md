# Progress

THawk2.exe holds 4720 code functions (959,739 bytes). Each function is attributed to its original source file through the assert strings the executable still carries; functions without one take the file of their neighbours when both sides agree.

`implemented` means the routine is reimplemented in Rust and checked frame by frame, against traces of the original game where a recorded case reaches it and against an independent model of the recovered behaviour otherwise. `recovered` means the behaviour has been read from the binary and documented but not yet reimplemented. Everything else is open.

## Overall

| | Functions | Bytes |
|---|---|---|
| Implemented | 388 (8.2%) | 218,389 (22.8%) |
| Recovered | 209 (4.4%) | 77,844 (8.1%) |
| Open | 4123 | 663,506 |

`█████░░░░░░░░░░░░░░░` 22.8% implemented by bytes

## By original source file

| Source file | Functions | Bytes | Implemented | Recovered | Progress |
|---|---|---|---|---|---|
| unattributed | 2977 | 354,046 | 127 | 92 | `█░░░░░░░░░` 13% |
| physics.cpp | 110 | 64,313 | 49 | 36 | `███████░░░` 73% |
| mmu.cpp | 70 | 31,238 | 2 | 9 | `░░░░░░░░░░` 3% |
| front.cpp | 42 | 29,072 | 5 | 3 | `██░░░░░░░░` 20% |
| bit.cpp | 123 | 25,393 | 9 | 3 | `█░░░░░░░░░` 9% |
| LevelEditor.cpp | 56 | 23,929 | 0 | 0 | `░░░░░░░░░░` 0% |
| LevelGen.cpp | 38 | 23,037 | 0 | 0 | `░░░░░░░░░░` 0% |
| options.cpp | 54 | 22,299 | 0 | 1 | `░░░░░░░░░░` 0% |
| create.cpp | 54 | 18,174 | 3 | 0 | `█░░░░░░░░░` 12% |
| GenObjs.cpp | 58 | 15,497 | 0 | 0 | `░░░░░░░░░░` 0% |
| panel.cpp | 37 | 14,181 | 18 | 1 | `██████░░░░` 64% |
| career.cpp | 81 | 13,988 | 38 | 9 | `█████░░░░░` 48% |
| trig.cpp | 30 | 13,716 | 13 | 2 | `███████░░░` 73% |
| main.cpp | 29 | 13,255 | 7 | 10 | `██░░░░░░░░` 22% |
| spool.cpp | 41 | 12,775 | 7 | 0 | `████░░░░░░` 36% |
| Menu.cpp | 51 | 10,954 | 0 | 0 | `░░░░░░░░░░` 0% |
| competition.cpp | 30 | 10,871 | 0 | 2 | `░░░░░░░░░░` 0% |
| playsel.cpp | 37 | 10,008 | 0 | 0 | `░░░░░░░░░░` 0% |
| video.cpp | 28 | 10,005 | 4 | 14 | `██░░░░░░░░` 23% |
| d3dtex.cpp | 20 | 9,174 | 8 | 0 | `██████░░░░` 57% |
| EquipSel.cpp | 44 | 8,492 | 0 | 0 | `░░░░░░░░░░` 0% |
| trickwin.cpp | 17 | 8,388 | 0 | 0 | `░░░░░░░░░░` 0% |
| camera.cpp | 22 | 8,285 | 4 | 0 | `████████░░` 77% |
| m3d.cpp | 16 | 7,875 | 9 | 0 | `███████░░░` 73% |
| camera.h | 1 | 7,297 | 1 | 0 | `██████████` 100% |
| NetGame.cpp | 23 | 6,946 | 0 | 0 | `░░░░░░░░░░` 0% |
| cresel.cpp | 8 | 6,893 | 0 | 0 | `░░░░░░░░░░` 0% |
| baddy.cpp | 10 | 6,545 | 5 | 0 | `████████░░` 80% |
| trickdb.cpp | 22 | 6,398 | 13 | 2 | `█████████░` 89% |
| FrontEnd2.cpp | 10 | 6,381 | 0 | 2 | `░░░░░░░░░░` 0% |
| WinMain.cpp | 13 | 5,956 | 0 | 3 | `░░░░░░░░░░` 0% |
| appwin.cpp | 12 | 5,751 | 0 | 0 | `░░░░░░░░░░` 0% |
| m3dcolij.cpp | 6 | 5,728 | 2 | 1 | `███████░░░` 71% |
| cre_texset.cpp | 4 | 5,005 | 3 | 0 | `██████████` 95% |
| PCMovie.cpp | 7 | 4,977 | 0 | 0 | `░░░░░░░░░░` 0% |
| trickscript.cpp | 2 | 4,585 | 1 | 0 | `█████████░` 95% |
| create_stitch.cpp | 15 | 4,319 | 0 | 0 | `░░░░░░░░░░` 0% |
| ob.cpp | 29 | 4,297 | 8 | 3 | `█████░░░░░` 50% |
| soundlib.cpp | 14 | 4,264 | 0 | 0 | `░░░░░░░░░░` 0% |
| Image.cpp | 32 | 4,213 | 2 | 0 | `██░░░░░░░░` 21% |
| powerup.cpp | 14 | 4,208 | 6 | 0 | `████████░░` 78% |
| statwin.cpp | 13 | 4,153 | 0 | 0 | `░░░░░░░░░░` 0% |
| FontTools.cpp | 20 | 4,086 | 8 | 1 | `██████░░░░` 56% |
| dust.cpp | 7 | 3,770 | 7 | 0 | `██████████` 100% |
| commonfe.cpp | 26 | 3,710 | 0 | 0 | `░░░░░░░░░░` 0% |
| rail.cpp | 14 | 3,610 | 5 | 2 | `███████░░░` 73% |
| CreEdit.cpp | 9 | 3,519 | 0 | 0 | `░░░░░░░░░░` 0% |
| platform.cpp | 4 | 3,463 | 3 | 0 | `██████████` 97% |
| skatmgr.cpp | 30 | 3,436 | 1 | 2 | `░░░░░░░░░░` 4% |
| DxPlay.cpp | 24 | 2,986 | 0 | 0 | `░░░░░░░░░░` 0% |
| decomp.cpp | 2 | 2,930 | 2 | 0 | `██████████` 100% |
| mem.cpp | 11 | 2,904 | 0 | 0 | `░░░░░░░░░░` 0% |
| player_spool.cpp | 11 | 2,885 | 0 | 0 | `░░░░░░░░░░` 0% |
| statedit.cpp | 7 | 2,740 | 0 | 0 | `░░░░░░░░░░` 0% |
| fileio.cpp | 11 | 2,547 | 0 | 0 | `░░░░░░░░░░` 0% |
| EditorGlobal.cpp | 14 | 2,379 | 0 | 0 | `░░░░░░░░░░` 0% |
| m3dzone.cpp | 2 | 2,370 | 1 | 0 | `████████░░` 77% |
| Credits.cpp | 7 | 2,201 | 0 | 0 | `░░░░░░░░░░` 0% |
| mech.cpp | 9 | 2,197 | 4 | 1 | `██░░░░░░░░` 21% |
| Spooler.cpp | 17 | 2,191 | 0 | 0 | `░░░░░░░░░░` 0% |
| d3dpoly.cpp | 7 | 2,138 | 0 | 0 | `░░░░░░░░░░` 0% |
| create_color.cpp | 5 | 2,129 | 0 | 0 | `░░░░░░░░░░` 0% |
| pal.cpp | 12 | 2,059 | 1 | 0 | `████░░░░░░` 36% |
| m3dutils.cpp | 4 | 1,997 | 0 | 0 | `░░░░░░░░░░` 0% |
| FaceRemoval.cpp | 3 | 1,841 | 0 | 0 | `░░░░░░░░░░` 0% |
| Shatter.cpp | 1 | 1,689 | 1 | 0 | `██████████` 100% |
| tricksel.cpp | 3 | 1,634 | 0 | 0 | `░░░░░░░░░░` 0% |
| screentools.cpp | 10 | 1,625 | 0 | 0 | `░░░░░░░░░░` 0% |
| Mess.cpp | 13 | 1,614 | 0 | 0 | `░░░░░░░░░░` 0% |
| d3dfunc.cpp | 7 | 1,614 | 0 | 0 | `░░░░░░░░░░` 0% |
| player_ui.cpp | 5 | 1,590 | 0 | 0 | `░░░░░░░░░░` 0% |
| redbook.cpp | 10 | 1,462 | 0 | 0 | `░░░░░░░░░░` 0% |
| trickob.cpp | 6 | 1,419 | 0 | 3 | `░░░░░░░░░░` 0% |
| texteffects.cpp | 17 | 1,332 | 7 | 0 | `████░░░░░░` 43% |
| create_ui.cpp | 6 | 1,325 | 0 | 0 | `░░░░░░░░░░` 0% |
| PRE.cpp | 9 | 1,246 | 0 | 0 | `░░░░░░░░░░` 0% |
| d3dsprite.cpp | 2 | 1,243 | 0 | 0 | `░░░░░░░░░░` 0% |
| bit2.cpp | 8 | 1,144 | 0 | 0 | `░░░░░░░░░░` 0% |
| m3dinit.cpp | 3 | 1,138 | 1 | 0 | `██████░░░░` 62% |
| db.cpp | 3 | 1,116 | 0 | 0 | `░░░░░░░░░░` 0% |
| DynamicLogo.cpp | 3 | 1,088 | 2 | 0 | `████████░░` 80% |
| pad.cpp | 8 | 1,043 | 0 | 1 | `░░░░░░░░░░` 0% |
| carsel.cpp | 5 | 1,032 | 0 | 0 | `░░░░░░░░░░` 0% |
| perwin.cpp | 4 | 995 | 0 | 0 | `░░░░░░░░░░` 0% |
| goalwin.cpp | 1 | 979 | 0 | 0 | `░░░░░░░░░░` 0% |
| PcInput.cpp | 1 | 970 | 0 | 1 | `░░░░░░░░░░` 0% |
| tapes.cpp | 1 | 803 | 0 | 0 | `░░░░░░░░░░` 0% |
| custmgr.cpp | 2 | 765 | 0 | 0 | `░░░░░░░░░░` 0% |
| videowin.cpp | 1 | 756 | 0 | 0 | `░░░░░░░░░░` 0% |
| EditorPieces2.cpp | 6 | 737 | 0 | 0 | `░░░░░░░░░░` 0% |
| medals.cpp | 1 | 660 | 0 | 0 | `░░░░░░░░░░` 0% |
| LevelSel.cpp | 1 | 646 | 0 | 0 | `░░░░░░░░░░` 0% |
| cre_tex.cpp | 2 | 645 | 0 | 0 | `░░░░░░░░░░` 0% |
| init.cpp | 1 | 600 | 0 | 0 | `░░░░░░░░░░` 0% |
| car.cpp | 1 | 570 | 0 | 0 | `░░░░░░░░░░` 0% |
| MainMenu.cpp | 2 | 511 | 0 | 1 | `░░░░░░░░░░` 0% |
| tricktips.cpp | 2 | 499 | 0 | 2 | `░░░░░░░░░░` 0% |
| sfx.cpp | 3 | 493 | 0 | 0 | `░░░░░░░░░░` 0% |
| D3dTimer.cpp | 2 | 424 | 0 | 2 | `░░░░░░░░░░` 0% |
| NetMenu.cpp | 1 | 341 | 0 | 0 | `░░░░░░░░░░` 0% |
| cresave.cpp | 1 | 333 | 0 | 0 | `░░░░░░░░░░` 0% |
| vidmenu.cpp | 1 | 319 | 0 | 0 | `░░░░░░░░░░` 0% |
| thstub.cpp | 1 | 312 | 0 | 0 | `░░░░░░░░░░` 0% |
| multisel.cpp | 1 | 301 | 0 | 0 | `░░░░░░░░░░` 0% |
| exp.cpp | 1 | 289 | 1 | 0 | `██████████` 100% |
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
| 004013b0 | 29 | baddy.cpp | objects.rs |
| 00401520 | 177 | baddy.cpp | objects.rs |
| 004015e0 | 4187 | baddy.cpp | objects.rs |
| 004027c0 | 35 | baddy.cpp | objects.rs |
| 00402bb0 | 815 | baddy.cpp | objects.rs |
| 00403000 | 171 | unattributed | objects.rs |
| 00403410 | 511 | unattributed | objects.rs |
| 00403f00 | 71 | bit.cpp | ground.rs |
| 00403f70 | 20 | bit.cpp | ground.rs |
| 004051c0 | 458 | bit.cpp | collision.rs |
| 00406200 | 255 | bit.cpp | ground.rs |
| 00406330 | 171 | bit.cpp | ground.rs |
| 004069a0 | 365 | bit.cpp | collision.rs |
| 00406d00 | 171 | bit.cpp | collision.rs |
| 00406e30 | 474 | bit.cpp | collision.rs |
| 00407050 | 384 | bit.cpp | collision.rs |
| 0040b060 | 315 | unattributed | grind.rs |
| 0040b650 | 1409 | camera.cpp | camera.rs |
| 0040bd40 | 249 | camera.cpp | camera.rs |
| 0040be70 | 508 | unattributed | camera.rs |
| 0040c370 | 7297 | camera.h | camera.rs |
| 0040e090 | 3294 | camera.cpp | camera.rs |
| 00410610 | 1431 | camera.cpp | camera.rs |
| 004138f0 | 791 | career.cpp | fresh.rs |
| 00413c10 | 325 | career.cpp | fresh.rs |
| 00413f30 | 63 | career.cpp | fresh.rs |
| 00413f70 | 183 | career.cpp | fresh.rs |
| 004141a0 | 33 | career.cpp | career.rs |
| 004141d0 | 168 | career.cpp | career.rs |
| 00414280 | 84 | career.cpp | career.rs |
| 004142e0 | 608 | career.cpp | career.rs |
| 00414540 | 47 | career.cpp | career.rs |
| 004145e0 | 106 | career.cpp | career.rs |
| 00414650 | 31 | career.cpp | career.rs |
| 00414810 | 217 | career.cpp | career.rs |
| 004148f0 | 217 | career.cpp | career.rs |
| 004149d0 | 126 | career.cpp | career.rs |
| 00414a50 | 38 | career.cpp | career.rs |
| 00414a80 | 38 | career.cpp | career.rs |
| 00414ab0 | 35 | career.cpp | career.rs |
| 00414ae0 | 35 | career.cpp | career.rs |
| 00414b50 | 51 | career.cpp | career.rs |
| 00414bb0 | 15 | career.cpp | career.rs |
| 00414bc0 | 15 | career.cpp | career.rs |
| 00414bd0 | 125 | career.cpp | career.rs |
| 00414c50 | 312 | career.cpp | career.rs |
| 00414d90 | 518 | career.cpp | career.rs |
| 00414fa0 | 179 | career.cpp | career.rs |
| 00415060 | 275 | career.cpp | career.rs |
| 00415180 | 353 | career.cpp | career.rs |
| 004152f0 | 143 | career.cpp | career.rs |
| 004158d0 | 54 | career.cpp | career.rs |
| 00415910 | 183 | career.cpp | career.rs |
| 004159d0 | 191 | career.cpp | career.rs |
| 00415a90 | 69 | career.cpp | career.rs |
| 00416050 | 19 | career.cpp | career.rs |
| 00416140 | 236 | career.cpp | input.rs |
| 00416230 | 264 | career.cpp | input.rs |
| 00416380 | 300 | career.cpp | input.rs |
| 00416740 | 55 | career.cpp | career.rs |
| 004170d0 | 258 | career.cpp | fresh.rs |
| 0041db50 | 105 | unattributed | create.rs |
| 0041dc60 | 78 | unattributed | create.rs |
| 0041ddb0 | 228 | cre_texset.cpp | create.rs |
| 0041dea0 | 4264 | cre_texset.cpp | create.rs |
| 0041ef50 | 272 | cre_texset.cpp | create.rs |
| 00422f20 | 1097 | create.cpp | create.rs |
| 00423370 | 850 | create.cpp | create.rs |
| 004236d0 | 302 | create.cpp | fresh.rs |
| 004305f0 | 773 | decomp.cpp | pose.rs |
| 00430920 | 2157 | decomp.cpp | pose.rs |
| 00431260 | 192 | dust.cpp | dust.rs |
| 00431320 | 810 | dust.cpp | dust.rs |
| 00431650 | 362 | dust.cpp | dust.rs |
| 004317c0 | 216 | dust.cpp | dust.rs |
| 004318a0 | 724 | dust.cpp | dust.rs |
| 00431b80 | 733 | dust.cpp | dust.rs |
| 00431e80 | 733 | dust.cpp | dust.rs |
| 004321d0 | 117 | DynamicLogo.cpp | materials.rs |
| 00432330 | 752 | DynamicLogo.cpp | materials.rs |
| 00448860 | 289 | exp.cpp | collision.rs |
| 00449cd0 | 906 | unattributed | font.rs |
| 0044a070 | 1100 | FontTools.cpp | text.rs |
| 0044a4d0 | 104 | FontTools.cpp | text.rs |
| 0044a540 | 33 | FontTools.cpp | text.rs |
| 0044a570 | 75 | FontTools.cpp | text.rs |
| 0044a5c0 | 88 | FontTools.cpp | text.rs |
| 0044a620 | 574 | FontTools.cpp | text.rs |
| 0044a860 | 119 | FontTools.cpp | text.rs |
| 0044aaa0 | 199 | FontTools.cpp | text.rs |
| 0044b720 | 67 | front.cpp | hud.rs |
| 0044ba90 | 296 | front.cpp | hud.rs |
| 0044ca60 | 4737 | front.cpp | hud.rs |
| 0044e0f0 | 756 | front.cpp | career.rs |
| 0044e3f0 | 41 | front.cpp | hud.rs |
| 004568d0 | 185 | unattributed | text.rs |
| 00457160 | 628 | Image.cpp | text.rs |
| 00457420 | 244 | Image.cpp | sprite.rs |
| 0045e8e0 | 1690 | unattributed | render.rs |
| 0045f530 | 4024 | unattributed | render.rs |
| 004604f0 | 1429 | m3d.cpp | figure.rs |
| 00460a90 | 500 | m3d.cpp | figure.rs |
| 00460ca0 | 1092 | m3d.cpp | draw.rs |
| 004610f0 | 2388 | m3d.cpp | draw.rs |
| 00461a50 | 50 | m3d.cpp | render.rs |
| 00461a90 | 50 | m3d.cpp | projection.rs |
| 00461b10 | 64 | m3d.cpp | figure.rs |
| 00461b70 | 50 | m3d.cpp | projection.rs |
| 00461bf0 | 158 | m3d.cpp | figure.rs |
| 004624d0 | 1046 | unattributed | collision.rs |
| 00462a20 | 2903 | m3dcolij.cpp | collision.rs |
| 004638d0 | 1143 | m3dcolij.cpp | collision.rs |
| 00464200 | 1004 | unattributed | pickups.rs |
| 004647c0 | 704 | m3dinit.cpp | draw.rs |
| 00465830 | 1831 | unattributed | rail.rs |
| 00465f60 | 201 | unattributed | skater.rs |
| 00466090 | 21 | unattributed | collision.rs |
| 004660b0 | 1827 | m3dzone.cpp | collision.rs |
| 00466c10 | 455 | unattributed | session.rs |
| 004674d0 | 307 | main.cpp | materials.rs |
| 00468b00 | 35 | main.cpp | ground.rs |
| 00468b30 | 444 | main.cpp | ground.rs |
| 00469a30 | 764 | main.cpp | session.rs |
| 00469d30 | 175 | main.cpp | session.rs |
| 00469de0 | 774 | main.cpp | ground.rs |
| 0046a3a0 | 352 | main.cpp | ground.rs |
| 0046c5f0 | 62 | unattributed | ground.rs |
| 0046c630 | 64 | unattributed | ground.rs |
| 0046c670 | 167 | unattributed | ground.rs |
| 0046cdf0 | 44 | mech.cpp | score.rs |
| 0046ce20 | 62 | mech.cpp | score.rs |
| 0046ce70 | 172 | mech.cpp | score.rs |
| 0046cf20 | 191 | mech.cpp | score.rs |
| 0046d1d0 | 77 | unattributed | collision.rs |
| 0046d220 | 189 | unattributed | collision.rs |
| 0046d2e0 | 938 | unattributed | collision.rs |
| 0046d7a0 | 416 | unattributed | ground.rs |
| 0046d970 | 252 | unattributed | ground.rs |
| 0046da70 | 684 | unattributed | ground.rs |
| 0046dd20 | 61 | unattributed | ground.rs |
| 0046e1d0 | 106 | unattributed | objects.rs |
| 00475700 | 305 | mmu.cpp | card.rs |
| 0047b400 | 730 | mmu.cpp | rec.rs |
| 004801d0 | 31 | ob.cpp | ground.rs |
| 004802f0 | 181 | ob.cpp | pickups.rs |
| 004803b0 | 38 | ob.cpp | pickups.rs |
| 00480430 | 37 | ob.cpp | pickups.rs |
| 00480460 | 361 | ob.cpp | pickups.rs |
| 00480730 | 340 | ob.cpp | anim.rs |
| 00480950 | 709 | ob.cpp | anim.rs |
| 00480d90 | 436 | ob.cpp | figure.rs |
| 00481150 | 119 | unattributed | pickups.rs |
| 0048a7e0 | 749 | pal.cpp | materials.rs |
| 0048b760 | 371 | panel.cpp | hud.rs |
| 0048bae0 | 1037 | panel.cpp | hud.rs |
| 0048bef0 | 447 | panel.cpp | hud.rs |
| 0048c1a0 | 887 | panel.cpp | tricktext.rs |
| 0048c520 | 562 | panel.cpp | hud.rs |
| 0048cb10 | 74 | panel.cpp | tricktext.rs |
| 0048cb60 | 94 | panel.cpp | score.rs |
| 0048cbc0 | 374 | panel.cpp | score.rs |
| 0048cd40 | 59 | panel.cpp | score.rs |
| 0048cd80 | 673 | panel.cpp | tricktext.rs |
| 0048d090 | 1455 | panel.cpp | hud.rs |
| 0048d710 | 135 | panel.cpp | manual.rs |
| 0048d7c0 | 104 | panel.cpp | pickups.rs |
| 0048d8a0 | 183 | panel.cpp | career.rs |
| 0048ddc0 | 1346 | panel.cpp | tricktext.rs |
| 0048e320 | 142 | panel.cpp | tricktext.rs |
| 0048e3b0 | 623 | panel.cpp | score.rs |
| 0048e680 | 538 | panel.cpp | score.rs |
| 0048e8a0 | 163 | unattributed | score.rs |
| 0048ea80 | 131 | unattributed | collision.rs |
| 0048eb70 | 59 | unattributed | wall.rs |
| 0048f3a0 | 491 | physics.cpp | ground.rs |
| 0048f690 | 131 | physics.cpp | ground.rs |
| 0048f720 | 152 | physics.cpp | manual.rs |
| 004900b0 | 549 | physics.cpp | ground.rs |
| 004902e0 | 267 | physics.cpp | ground.rs |
| 004903f0 | 40 | physics.cpp | anim.rs |
| 00490420 | 43 | physics.cpp | anim.rs |
| 00490450 | 46 | physics.cpp | anim.rs |
| 00490480 | 49 | physics.cpp | anim.rs |
| 004904d0 | 311 | physics.cpp | ground.rs |
| 00490610 | 110 | physics.cpp | ground.rs |
| 00490680 | 169 | physics.cpp | ground.rs |
| 00490730 | 932 | physics.cpp | ground.rs |
| 00490ef0 | 1482 | physics.cpp | grind.rs |
| 004914d0 | 502 | physics.cpp | grind.rs |
| 00491780 | 1010 | physics.cpp | ground.rs |
| 00491c10 | 124 | physics.cpp | grind.rs |
| 00492290 | 367 | physics.cpp | manual.rs |
| 004925e0 | 1195 | physics.cpp | manual.rs |
| 00492a90 | 131 | physics.cpp | script.rs |
| 00492ed0 | 70 | physics.cpp | ground.rs |
| 00492f20 | 1080 | physics.cpp | ground.rs |
| 00493370 | 2707 | physics.cpp | ground.rs |
| 00493f40 | 212 | physics.cpp | grind.rs |
| 00494180 | 141 | physics.cpp | grind.rs |
| 00494210 | 5330 | physics.cpp | grind.rs |
| 004956f0 | 197 | physics.cpp | ground.rs |
| 004957c0 | 802 | physics.cpp | ground.rs |
| 00496060 | 534 | physics.cpp | skater.rs |
| 00496300 | 96 | physics.cpp | ground.rs |
| 00496360 | 491 | physics.cpp | ground.rs |
| 00496550 | 5117 | physics.cpp | ground.rs |
| 00497960 | 306 | physics.cpp | ground.rs |
| 00497f40 | 5285 | physics.cpp | ground.rs |
| 004995d0 | 305 | physics.cpp | grind.rs |
| 0049a280 | 3240 | physics.cpp | ground.rs |
| 0049b010 | 1224 | physics.cpp | ground.rs |
| 0049b500 | 1477 | physics.cpp | ground.rs |
| 0049bad0 | 1421 | physics.cpp | ground.rs |
| 0049c060 | 716 | physics.cpp | ground.rs |
| 0049c330 | 1172 | physics.cpp | ground.rs |
| 0049c7d0 | 120 | physics.cpp | ground.rs |
| 0049d080 | 612 | physics.cpp | ground.rs |
| 0049d480 | 690 | physics.cpp | ground.rs |
| 0049d9c0 | 271 | physics.cpp | ground.rs |
| 0049db80 | 826 | physics.cpp | ground.rs |
| 0049df00 | 1296 | physics.cpp | ground.rs |
| 0049e480 | 229 | physics.cpp | session.rs |
| 0049e680 | 2983 | physics.cpp | ground.rs |
| 0049f250 | 235 | unattributed | objects.rs |
| 0049f4c0 | 669 | platform.cpp | objects.rs |
| 0049f780 | 1875 | platform.cpp | objects.rs |
| 004a03e0 | 802 | platform.cpp | objects.rs |
| 004a1060 | 469 | unattributed | objects.rs |
| 004a12d0 | 354 | unattributed | objects.rs |
| 004a7c50 | 1275 | powerup.cpp | pickups.rs |
| 004a8290 | 36 | powerup.cpp | pickups.rs |
| 004a8300 | 789 | powerup.cpp | pickups.rs |
| 004a8620 | 195 | powerup.cpp | pickups.rs |
| 004a86f0 | 842 | powerup.cpp | pickups.rs |
| 004a8ac0 | 140 | powerup.cpp | pickups.rs |
| 004a9650 | 255 | unattributed | camera.rs |
| 004a9820 | 71 | unattributed | camera.rs |
| 004a9870 | 71 | unattributed | camera.rs |
| 004a9910 | 237 | unattributed | camera.rs |
| 004a9a00 | 491 | unattributed | camera.rs |
| 004a9bf0 | 890 | unattributed | camera.rs |
| 004a9f70 | 81 | unattributed | rail.rs |
| 004a9fd0 | 17 | unattributed | rail.rs |
| 004aa040 | 79 | unattributed | rail.rs |
| 004aa220 | 98 | rail.cpp | rail.rs |
| 004aa290 | 148 | rail.cpp | rail.rs |
| 004aa330 | 135 | rail.cpp | rail.rs |
| 004aa5d0 | 663 | rail.cpp | rail.rs |
| 004aa8c0 | 1608 | rail.cpp | rail.rs |
| 004aaf70 | 2622 | unattributed | rail.rs |
| 004add10 | 336 | unattributed | collision.rs |
| 004adee0 | 311 | unattributed | collision.rs |
| 004ae020 | 97 | unattributed | collision.rs |
| 004ae090 | 1689 | Shatter.cpp | collision.rs |
| 004ae730 | 1980 | unattributed | collision.rs |
| 004af0b0 | 212 | unattributed | collision.rs |
| 004af1c0 | 1124 | unattributed | collision.rs |
| 004b1810 | 122 | skatmgr.cpp | fresh.rs |
| 004b1ef0 | 68 | spool.cpp | collision.rs |
| 004b20f0 | 33 | spool.cpp | psx.rs |
| 004b2450 | 1578 | spool.cpp | anim.rs |
| 004b2b70 | 1020 | spool.cpp | bitmap.rs |
| 004b39b0 | 1088 | spool.cpp | materials.rs |
| 004b43e0 | 602 | spool.cpp | materials.rs |
| 004b46a0 | 247 | spool.cpp | materials.rs |
| 004b82b0 | 137 | unattributed | career.rs |
| 004b8340 | 31 | unattributed | input.rs |
| 004b9720 | 8 | texteffects.cpp | tricktext.rs |
| 004b9790 | 132 | texteffects.cpp | tricktext.rs |
| 004b9820 | 56 | texteffects.cpp | tricktext.rs |
| 004b9d50 | 13 | texteffects.cpp | tricktext.rs |
| 004b9d60 | 119 | texteffects.cpp | tricktext.rs |
| 004b9de0 | 26 | texteffects.cpp | tricktext.rs |
| 004b9e00 | 225 | texteffects.cpp | tricktext.rs |
| 004ba780 | 21 | unattributed | tricktext.rs |
| 004bb4f0 | 602 | trickdb.cpp | input.rs |
| 004bb7e0 | 493 | trickdb.cpp | input.rs |
| 004bba50 | 581 | trickdb.cpp | input.rs |
| 004bbd70 | 386 | trickdb.cpp | input.rs |
| 004bbf00 | 1017 | trickdb.cpp | input.rs |
| 004bc330 | 105 | trickdb.cpp | input.rs |
| 004bc3a0 | 475 | trickdb.cpp | input.rs |
| 004bc7e0 | 280 | trickdb.cpp | input.rs |
| 004bc900 | 272 | trickdb.cpp | input.rs |
| 004bcb70 | 278 | trickdb.cpp | input.rs |
| 004bcc90 | 306 | trickdb.cpp | input.rs |
| 004bcdd0 | 289 | trickdb.cpp | input.rs |
| 004bcf00 | 595 | trickdb.cpp | input.rs |
| 004bd1e0 | 141 | unattributed | input.rs |
| 004be3e0 | 102 | unattributed | ground.rs |
| 004be450 | 4350 | trickscript.cpp | ground.rs |
| 004c4ba0 | 44 | unattributed | ground.rs |
| 004c4bd0 | 114 | unattributed | ground.rs |
| 004c4d10 | 286 | trig.cpp | ground.rs |
| 004c5130 | 580 | trig.cpp | trig.rs |
| 004c5420 | 32 | trig.cpp | trig.rs |
| 004c5440 | 20 | trig.cpp | trig.rs |
| 004c5460 | 880 | trig.cpp | ground.rs |
| 004c58b0 | 498 | trig.cpp | ground.rs |
| 004c5b00 | 44 | trig.cpp | ground.rs |
| 004c5b30 | 44 | trig.cpp | ground.rs |
| 004c5b60 | 155 | trig.cpp | ground.rs |
| 004c5dc0 | 6201 | trig.cpp | trig.rs |
| 004c77f0 | 522 | trig.cpp | ground.rs |
| 004c7a00 | 489 | trig.cpp | ground.rs |
| 004c8650 | 316 | trig.cpp | trig.rs |
| 004c9320 | 30 | unattributed | rng.rs |
| 004c9500 | 136 | unattributed | camera.rs |
| 004c9620 | 170 | unattributed | pickups.rs |
| 004c9770 | 258 | unattributed | camera.rs |
| 004c9f50 | 95 | unattributed | trig.rs |
| 004ca0a0 | 32 | unattributed | camera.rs |
| 004ca2d0 | 846 | unattributed | collision.rs |
| 004ca8f0 | 73 | unattributed | camera.rs |
| 004ca9b0 | 61 | unattributed | objects.rs |
| 004ca9f0 | 41 | unattributed | skater.rs |
| 004caa20 | 41 | unattributed | skater.rs |
| 004caae0 | 39 | unattributed | rec.rs |
| 004cab10 | 55 | unattributed | objects.rs |
| 004cabb0 | 49 | unattributed | ground.rs |
| 004cabf0 | 49 | unattributed | skater.rs |
| 004cac30 | 46 | unattributed | skater.rs |
| 004cac90 | 54 | unattributed | ground.rs |
| 004cacd0 | 43 | unattributed | ground.rs |
| 004cad30 | 35 | unattributed | skater.rs |
| 004cad60 | 17 | unattributed | objects.rs |
| 004cad80 | 61 | unattributed | objects.rs |
| 004cae10 | 31 | unattributed | objects.rs |
| 004ccd70 | 1898 | video.cpp | rec.rs |
| 004cd5a0 | 226 | video.cpp | rec.rs |
| 004cd690 | 128 | video.cpp | rec.rs |
| 004cd710 | 53 | video.cpp | rec.rs |
| 004cf030 | 710 | unattributed | options.rs |
| 004cf300 | 274 | unattributed | options.rs |
| 004d0a20 | 22 | unattributed | materials.rs |
| 004d0a40 | 27 | unattributed | materials.rs |
| 004d11d0 | 763 | unattributed | figure.rs |
| 004d14d0 | 988 | unattributed | render.rs |
| 004d18b0 | 175 | unattributed | render.rs |
| 004d1960 | 873 | unattributed | render.rs |
| 004d1d40 | 936 | unattributed | render.rs |
| 004d20f0 | 530 | unattributed | render.rs |
| 004d2310 | 688 | unattributed | render.rs |
| 004d26b0 | 797 | unattributed | render.rs |
| 004d29e0 | 959 | unattributed | render.rs |
| 004d3600 | 188 | unattributed | text.rs |
| 004d4120 | 117 | unattributed | text.rs |
| 004d8cd0 | 285 | d3dtex.cpp | bitmap.rs |
| 004d8df0 | 273 | d3dtex.cpp | materials.rs |
| 004d8f10 | 1010 | d3dtex.cpp | bitmap.rs |
| 004d9310 | 794 | d3dtex.cpp | materials.rs |
| 004d9630 | 592 | d3dtex.cpp | font.rs |
| 004d9950 | 1465 | d3dtex.cpp | sprite.rs |
| 004d9f50 | 770 | d3dtex.cpp | materials.rs |
| 004da3e0 | 23 | d3dtex.cpp | materials.rs |
| 004da680 | 242 | unattributed | bitmap.rs |
| 004da780 | 184 | unattributed | font.rs |
| 004da840 | 666 | unattributed | materials.rs |
| 004dc0c0 | 60 | unattributed | hud.rs |
| 004e2f80 | 104 | unattributed | camera.rs |
| 004e2ff0 | 113 | unattributed | skater.rs |
| 004e3130 | 160 | unattributed | skater.rs |
| 004e3ce0 | 59 | unattributed | rail.rs |
| 004e3e80 | 252 | unattributed | options.rs |
| 004e4d10 | 1872 | unattributed | options.rs |
| 004e5460 | 698 | unattributed | options.rs |
| 004e7c60 | 374 | unattributed | skater.rs |
| 004e7de0 | 377 | unattributed | skater.rs |
| 004e7f60 | 371 | unattributed | skater.rs |
| 004e80e0 | 85 | unattributed | skater.rs |
| 004e8580 | 25 | unattributed | camera.rs |
| 004e85a0 | 329 | unattributed | camera.rs |
| 004e8770 | 3 | unattributed | bitmap.rs |
| 004e87f0 | 72 | unattributed | text.rs |
| 004e9780 | 421 | unattributed | pkr.rs |
| 004e9940 | 568 | unattributed | pkr.rs |
| 004f3980 | 42 | unattributed | ground.rs |
| 004f39b0 | 42 | unattributed | ground.rs |
| 004f39f0 | 1339 | unattributed | render.rs |
| 004f4940 | 445 | unattributed | collision.rs |
| 004f5390 | 19 | unattributed | pose.rs |
| 004f53b0 | 44 | unattributed | skater.rs |
| 004f5f90 | 39 | unattributed | camera.rs |
| 004f5fc0 | 19 | unattributed | skater.rs |
| 004f6b90 | 203 | unattributed | options.rs |
| 004f6c60 | 63 | unattributed | options.rs |
| 004f6ca0 | 37 | unattributed | options.rs |
| 004f6cd0 | 44 | unattributed | options.rs |

## Recovered, not yet reimplemented

| Address | Size | Source file |
|---|---|---|
| 004011e0 | 82 | unattributed |
| 00403d30 | 139 | bit.cpp |
| 00403e40 | 82 | bit.cpp |
| 004053d0 | 220 | bit.cpp |
| 00411fc0 | 1621 | unattributed |
| 00413d60 | 357 | career.cpp |
| 00414770 | 61 | career.cpp |
| 00415380 | 477 | career.cpp |
| 00415560 | 48 | career.cpp |
| 00415590 | 15 | career.cpp |
| 004165d0 | 288 | career.cpp |
| 004166f0 | 67 | career.cpp |
| 00416980 | 127 | career.cpp |
| 00416f80 | 43 | career.cpp |
| 00417330 | 145 | unattributed |
| 00419420 | 57 | unattributed |
| 00419600 | 627 | competition.cpp |
| 0041b540 | 69 | competition.cpp |
| 0042f680 | 125 | unattributed |
| 0042f700 | 255 | unattributed |
| 0044ab70 | 24 | FontTools.cpp |
| 0044b1f0 | 66 | unattributed |
| 0044dfa0 | 326 | front.cpp |
| 0044eac0 | 13872 | front.cpp |
| 004524a0 | 1492 | front.cpp |
| 00452a80 | 637 | unattributed |
| 00452d30 | 139 | unattributed |
| 00452de0 | 28 | unattributed |
| 00452ff0 | 3985 | FrontEnd2.cpp |
| 004544a0 | 615 | FrontEnd2.cpp |
| 00458500 | 400 | unattributed |
| 0045c390 | 130 | unattributed |
| 004628f0 | 182 | unattributed |
| 00463d50 | 241 | m3dcolij.cpp |
| 004657c0 | 100 | unattributed |
| 00468cf0 | 405 | main.cpp |
| 00469400 | 412 | main.cpp |
| 004695a0 | 344 | main.cpp |
| 00469700 | 511 | main.cpp |
| 00469910 | 214 | main.cpp |
| 004699f0 | 52 | main.cpp |
| 0046a0f0 | 165 | main.cpp |
| 0046a1a0 | 168 | main.cpp |
| 0046a770 | 351 | main.cpp |
| 0046a8d0 | 1489 | main.cpp |
| 0046aee0 | 12 | unattributed |
| 0046aef0 | 421 | unattributed |
| 0046b720 | 329 | MainMenu.cpp |
| 0046c720 | 1212 | mech.cpp |
| 0046d040 | 285 | unattributed |
| 0046d6e0 | 58 | unattributed |
| 0046e080 | 333 | unattributed |
| 004758e0 | 2324 | mmu.cpp |
| 0047a0b0 | 363 | mmu.cpp |
| 0047a220 | 595 | mmu.cpp |
| 0047ae10 | 884 | mmu.cpp |
| 0047b1a0 | 297 | mmu.cpp |
| 0047b2d0 | 302 | mmu.cpp |
| 0047b6e0 | 228 | mmu.cpp |
| 0047b8d0 | 12 | mmu.cpp |
| 0047b8e0 | 232 | mmu.cpp |
| 00480890 | 192 | ob.cpp |
| 00480fa0 | 41 | ob.cpp |
| 00480ff0 | 273 | ob.cpp |
| 00487490 | 1911 | options.cpp |
| 004895c0 | 66 | pad.cpp |
| 00489930 | 88 | unattributed |
| 00489990 | 127 | unattributed |
| 00489a10 | 674 | unattributed |
| 0048d6d0 | 58 | panel.cpp |
| 0048ec00 | 1156 | unattributed |
| 0048f5f0 | 129 | physics.cpp |
| 0048f7c0 | 56 | physics.cpp |
| 0048f800 | 151 | physics.cpp |
| 0048f8a0 | 67 | physics.cpp |
| 0048f8f0 | 548 | physics.cpp |
| 0048fb20 | 189 | physics.cpp |
| 0048ffc0 | 71 | physics.cpp |
| 004904c0 | 11 | physics.cpp |
| 00490ae0 | 395 | physics.cpp |
| 00490c70 | 442 | physics.cpp |
| 004916d0 | 172 | physics.cpp |
| 00491b80 | 129 | physics.cpp |
| 00491c90 | 270 | physics.cpp |
| 00492030 | 234 | physics.cpp |
| 00492120 | 99 | physics.cpp |
| 00492190 | 255 | physics.cpp |
| 00492400 | 233 | physics.cpp |
| 00492560 | 118 | physics.cpp |
| 00492ea0 | 41 | physics.cpp |
| 00493e10 | 289 | physics.cpp |
| 00494090 | 213 | physics.cpp |
| 00495af0 | 457 | physics.cpp |
| 00495cc0 | 927 | physics.cpp |
| 00496280 | 101 | physics.cpp |
| 00497aa0 | 272 | physics.cpp |
| 00497bb0 | 568 | physics.cpp |
| 00497df0 | 321 | physics.cpp |
| 00499710 | 2710 | physics.cpp |
| 0049a1b0 | 41 | physics.cpp |
| 0049a1e0 | 98 | physics.cpp |
| 0049c850 | 127 | physics.cpp |
| 0049c970 | 1731 | physics.cpp |
| 0049d740 | 329 | physics.cpp |
| 0049d8a0 | 283 | physics.cpp |
| 0049dad0 | 168 | physics.cpp |
| 0049e570 | 268 | physics.cpp |
| 004aa410 | 16 | rail.cpp |
| 004aa420 | 16 | rail.cpp |
| 004ab9c0 | 21 | unattributed |
| 004ad620 | 303 | unattributed |
| 004ad9f0 | 17 | unattributed |
| 004b0cd0 | 101 | skatmgr.cpp |
| 004b1430 | 109 | skatmgr.cpp |
| 004baeb0 | 157 | unattributed |
| 004bb240 | 132 | unattributed |
| 004bb2d0 | 262 | unattributed |
| 004bc300 | 41 | trickdb.cpp |
| 004bd170 | 105 | trickdb.cpp |
| 004bd980 | 587 | trickob.cpp |
| 004bdbd0 | 106 | trickob.cpp |
| 004bdc40 | 192 | trickob.cpp |
| 004c4620 | 708 | unattributed |
| 004c48f0 | 96 | tricktips.cpp |
| 004c4950 | 403 | tricktips.cpp |
| 004c4b80 | 31 | unattributed |
| 004c4e30 | 715 | trig.cpp |
| 004c8550 | 220 | trig.cpp |
| 004c9300 | 1 | unattributed |
| 004c9340 | 77 | unattributed |
| 004c9390 | 107 | unattributed |
| 004ca0c0 | 120 | unattributed |
| 004cb360 | 20 | unattributed |
| 004cb380 | 57 | unattributed |
| 004cb6e0 | 292 | video.cpp |
| 004cb810 | 530 | video.cpp |
| 004cba30 | 384 | video.cpp |
| 004cbbb0 | 137 | video.cpp |
| 004cbc40 | 65 | video.cpp |
| 004cbce0 | 577 | video.cpp |
| 004cbf30 | 61 | video.cpp |
| 004cbf70 | 44 | video.cpp |
| 004cc0a0 | 52 | video.cpp |
| 004cc190 | 2731 | video.cpp |
| 004ccc40 | 47 | video.cpp |
| 004ccc70 | 254 | video.cpp |
| 004cd750 | 707 | video.cpp |
| 004cdc20 | 413 | video.cpp |
| 004cde70 | 11 | unattributed |
| 004cde80 | 107 | unattributed |
| 004cdef0 | 109 | unattributed |
| 004cf850 | 350 | unattributed |
| 004dabb0 | 298 | D3dTimer.cpp |
| 004dad80 | 126 | D3dTimer.cpp |
| 004dae00 | 96 | unattributed |
| 004de010 | 34 | unattributed |
| 004de4d0 | 46 | unattributed |
| 004de6d0 | 188 | unattributed |
| 004dece0 | 362 | unattributed |
| 004dee50 | 363 | unattributed |
| 004e2070 | 182 | unattributed |
| 004e2130 | 168 | unattributed |
| 004e2450 | 96 | unattributed |
| 004e2ef0 | 42 | unattributed |
| 004e3290 | 151 | unattributed |
| 004e4190 | 21 | unattributed |
| 004e42c0 | 901 | unattributed |
| 004e4650 | 55 | unattributed |
| 004e4690 | 970 | PcInput.cpp |
| 004e4a90 | 163 | unattributed |
| 004e7500 | 162 | unattributed |
| 004e75b0 | 803 | unattributed |
| 004e79f0 | 220 | unattributed |
| 004e7ad0 | 132 | unattributed |
| 004e7b60 | 87 | unattributed |
| 004e81a0 | 85 | unattributed |
| 004e8410 | 79 | unattributed |
| 004e8480 | 217 | unattributed |
| 004e8970 | 3 | unattributed |
| 004e9070 | 89 | unattributed |
| 004e9730 | 67 | unattributed |
| 004e9b80 | 164 | unattributed |
| 004ebf90 | 65 | unattributed |
| 004ec000 | 52 | unattributed |
| 004ec040 | 55 | unattributed |
| 004ec500 | 141 | unattributed |
| 004ec5c0 | 97 | unattributed |
| 004ec630 | 55 | unattributed |
| 004ec6b0 | 40 | unattributed |
| 004ec790 | 147 | unattributed |
| 004ec990 | 553 | unattributed |
| 004ecce0 | 194 | unattributed |
| 004ece80 | 198 | unattributed |
| 004ed2c0 | 180 | unattributed |
| 004ed380 | 49 | unattributed |
| 004ed7d0 | 84 | unattributed |
| 004f5f10 | 17 | unattributed |
| 004f5ff0 | 115 | unattributed |
| 004f6510 | 52 | unattributed |
| 004f6550 | 12 | unattributed |
| 004f7ce0 | 156 | WinMain.cpp |
| 004f7d80 | 1 | WinMain.cpp |
| 004f7e30 | 727 | WinMain.cpp |
| 005002a1 | 82 | unattributed |
| 005004f4 | 39 | unattributed |
| 00501740 | 25 | unattributed |
| 00502ccb | 67 | unattributed |
| 00502f74 | 233 | unattributed |
| 0050b7a0 | 208 | unattributed |

Status entries outside the function inventory: 0040b1d0, 0040f850, 00457940, 0046d160, 0049fef0, 004a07e0, 004b9860, 004b9b30, 004b9c80, 004ba070, 004ba430, 004ba450, 004ba480, 004ba760, 004d4bf0, 004d68b0
