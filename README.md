# ygopac

A tool for unpacking and repacking `.pac` files from the Yu-Gi-Oh! NDS games. It should work
for other games, especially the 2009 and 2010 games, but for now is only tested with Over the
Nexus.

There is a [python script](pacman.py) for people who don't want to use the rust version.

Both versions should give the same results, but the rust version is faster. If you have issues with
one try the other.

If you've never used [rust](https://www.rust-lang.org) you can compile the project like this:
```
cargo build --release
```
The binary should then be located at `target/release/ygopac[.exe]`.

If you're interested in how I made this, you can read the [spec](spec.md). I've basically figured
out everything written there and then implemented this tool. I was interested if other people had
done something similar, but the best I could find was [Nexus Revival](https://github.com/johnson-cooper/YGO-NEXUS-REVIVAL)
(which is a cool mod for Over the Nexus that you should check out!). The code they've used is not
as generic as this implementation.
