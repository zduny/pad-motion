# This was edited for use with MPR on Yuzu

### There is now a Video Tutorial!
https://www.youtube.com/watch?v=HVfgRrJXTEU<br />
[![Watch the video](https://img.youtube.com/vi/HVfgRrJXTEU/mqdefault.jpg)](https://www.youtube.com/watch?v=HVfgRrJXTEU)

# Pad-Motion
Implementation of Cemuhook gamepad motion protocol. 

Includes client and server.

https://crates.io/crates/pad-motion

## Gamepad and Mouse Server Example
You can download compiled example using first connected gamepad and mouse (used to emulate gyroscope data) as an input source, here:

[gamepad-and-mouse-server.zip](https://github.com/NarikoNep/pad-motion/releases/download/1.0.0/gamepad_and_mouse_server.zip)

If you need more help on how to run this example (and test if it's working), see [issue](https://github.com/zduny/pad-motion/issues/2).

## How to Set Up in Yuzu

Open `gamepad_and_mouse_server.exe` as administrator. It must be opened as administrator or the else the mouse will not be read. It also must be kept open at all times while being used with Yuzu. Open Yuzu and navitate to `Emulation > Configure > Controls` to set up your controls. We reccomend using the `Pro Controller` or `Handheld` as the emulated controller, as they only have one motion input. Near the bottom find `Motion 1`, click it, and shake the mouse until it says `cemuhookudp`. Your mouse has now been assigned as the motion device.

If this doesn't work, make sure the program is running as administator and navigate to the bottom of the Yuzu controller config window and find the motion checkbox. Ensure it is checked and press the `Configure` button under it. Under CemuhookUDP Config ensure Server is set as `127.0.0.1` and Port is set as `26760`. You can click `Test` to test if Yuzu is detecting your inputs.

(You also have to enable Gyro aiming in MPR's Settings)

![image](https://user-images.githubusercontent.com/97272732/219899284-1f9cb567-0999-45aa-9c35-12714ed89812.png)

**(Optional)** You can install [Metroid.ini](https://github.com/NarikoNep/pad-motion/releases/download/1.0.0/metroid.ini) from Releases into `%appdata%/yuzu/config/input` to get pre-created controls that closely resemble PrimeHack controls, no setup required. (This may need a tad bit of tweaking to be perfect for your use case)

Keep in mind Metroid.ini will automatically assign `cemuhookudp` as your Motion 1 input. If your mouse is not being detected, then try reassigning Motion 1.

To lock the mouse to the Yuzu Window you need to `Controls > Advanced > Enable mouse panning` and set it to 1%, to tweak controls afterward you need to press RightAlt and use the arrow keys to select the Options then you'll have your mouse back.

![image](https://user-images.githubusercontent.com/97272732/219899212-3addeed3-2f33-42d6-a3e0-bbd30164b8be.png)

## Use in Ryujinx
It is possible to use Pad Motion in Ryujinx, however it is more difficult due to Ryujinx not allowing you to use motion controls when the keyboard is your input source. 

In order to map motion controls with keyboard, you will have to download a keyboard to gamepad remapper such as reWASD. (If you have a Wooting keyboard, you can use Wootility)
Once you have set up your keyboard to gamepad remapper, open Ryujinx and navigate to `Options > Settings > Input > Player 1 Configure` and change `Input
 Device` to your emulated controller. On the bottom right the motion menu will appear. Check `Enable Motion Controls` and `Use CemuHook compatable motion`. Then set Server Host to `127.0.0.1` and Server Port to `26760`. You can also adjust the gyro sensitivity and deadzone here. Once you have set up your controls, save and close.
 
Run `gamepad_and_mouse_server.exe` as admin and run Metroid Prime Remastered and change the camera setting to stick + gyro and it should be working.

Unfortunately Ryujinx lacks Mouse Panning, which is useful for locking the mouse to Ryujinx's window. However, there is this program (https://github.com/IamSanjid/RMB) that will solve that for you.
 
 # How to Edit and Compile

**(Optional)** Download and install [Visual Studio Code](https://code.visualstudio.com/download)

Download and install [RustUp](https://www.rust-lang.org/tools/install) keep in mind your CPU architecture (32bit or 64bit)

Then open CMD and run these command to make sure they are all installed and able to run
```
rustc --version
cargo --version
rustup --version
```

Open the Examples folder inside the `pad-motion-master` folder, then open `gamepad-and-mouse-server.rs` in your prefered Text Editor/IDE

Then Scroll all the way to the bottom and edit the code with the sensitivity that you want.

```
gyroscope_pitch: -delta_rotation_y * (YOUR SENSITIVITY HERE),
gyroscope_roll: -delta_rotation_x * (YOUR SENSITIVITY HERE),
```

After it is edited Save the file then open your Command prompt back up then cd into the `pad-motion-master` folder then run this command

```
cargo build --examples --release
```

It should take about less than a Minute to build

Your built EXE will be in `pad-motion-master/target/release/examples` folder

It should be the first exe `gamepad_and_mouse_server.exe`

## Controls using Metroid.ini
```
WASD = Movement
Space = Jump/BoostBall
CTRL = Morph
Flick your Mouse up = Spring Ball
Left Click = Fire
Right Click = Lock-On
F = Fire Missles
1-2-3-4 = Change Visor
E + 1-2-3-4 = Change Beam
~ = Map
Enter = Pause Menu
```
## Projects using this Crate

[joycon-cemuhook](https://github.com/pickfire/joycon-cemuhook) by [pickfire](https://github.com/pickfire)

## References
[cemuhook-protocol](https://github.com/v1993/cemuhook-protocol)

## See also
[Cemuhook](https://cemuhook.sshnuke.net/)

[Cemuhook UDP Pad motion data provider setup](https://cemuhook.sshnuke.net/padudpserver.html)

[Cemu Emulator](https://cemu.info/)
