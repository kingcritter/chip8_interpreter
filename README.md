# README

This is my [Chip8](https://en.wikipedia.org/wiki/CHIP-8) emulator. There are many like it, but this one is mine. I wrote it following nothing more than [Cowgod's spec](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM). 

## Building and running

1. [install rust](https://www.rust-lang.org/tools/install)
2. install SDL2 development libraries for your system (on Fedora, it's `SDL2-devel`)
3. `cargo build`
4. `cargo run <path to game>`

## Controls

* The hex keypad is mapped to the left side of the keyboard
* `PgUp`/`PgDown` resizes the emulator
* The `pause`/`break` key pauses the game
  * while paused, `period` advances the game one tick and prints debug info to standard out
* `escape` exits the emulator

## Bugs

Resizing to a smaller size doesn't shrink the window along with the canvas.

Every game I've tried works, except for Tetris. In Tetris, if you move or rotate a piece, it snaps back after a second. I suspect this is do to input handling, but I can't see what I'm doing wrong.

Fuck Tetris.

# Update

[PhearTheCeal](https://github.com/PhearTheCeal) took a look at it and noticed I was doing an overflow addition where I shouldn't have, and my subtraction was backwards in one spot. Whoops. I'm honestly surprised everything else ran as expected. 

Now Tetris works. Man, what a great game.
