===============================
Rusty Snake
===============================

Welcome to Rusty Snake!

This is a game developed by me as an exercise to learn Rust.You're free to do what you want with it, perhaps learn some Rust?

Pre-requisites
==============

To build the game, the assets (sprites, sounds, musc) need to be downloaded separately. This is the asset package for the CleanCut Rusty Engine.

Make sure you are in the repository root, and use the following command:

    curl -L https://github.com/CleanCut/rusty_engine/archive/refs/heads/main.tar.gz | tar -zxv --strip-components=1 rusty_engine-main/assets

Building and run
================

It's the usual command:

    cargo run --release

How to play
===========

The game will place out pills for the snake to eat (blue sprites), and some moving objects to crash into (red sprites). Each pill will make your snake longer, and will earn you points. It can be played with up to four players.

There are only two buttons to control the snake, steer left and steer right. You start the game by pressing one of the buttons, which will make your snake to travel to the right, from approximately the middle of the screen.

==== ======
Keys Player
==== ======
q w  0
f g  1
u i  2
k l  3
==== ====== ========
