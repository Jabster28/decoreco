# decoreco

decode and re-encode big media files to save space

## usage examples

```bash
# re-encode all video files in your downloads folder to h264 and aac
decoreco ~/Downloads

# re-encode all video files in your downloads folder to hevc and mp3
decoreco -v hevc -a mp3 ~/Downloads

# list all video files in your home folder and sort them by size
decoreco -l -s ~/

# perform a dry run of converting your movies folder to avi
decoreco -d -v avi ~/Movies
```

## installation

macOS or linux using [brew](https://brew.sh):

```bash
brew install jabster28/jabster28/decoreco
```

building latest release with [cargo](https://doc.rust-lang.org/cargo/)

```bash
# rust should be installed by installing rustup from your favourite package manger e.g pacman -S rustup 
cargo install decoreco

# if cargo complains about binaries not being in PATH, add this to your shell profile:
# export PATH=$PATH:$HOME/.cargo/bin
```
