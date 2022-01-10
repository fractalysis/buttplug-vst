
## Running

This VST is assuming there is some kind of Intiface server already running, so install https://intiface.com/desktop/ first, and make sure the server is running. Once you open the plugin, any toys in range should automatically connect, but the behaviour might be unpredictable if there is more than one toy or if the toy doesn't vibrate.

Each song may sound better with different parameters:
### Low / High Frequency
These select the band of frequencies which are able to affect the buttplug, this vst will find the loudest frequency and use it as the vibration speed
### Cutoff
If the loudest frequency is quieter than this value compared to the rest of the song, the vibrator will be shut off


## Building

This VST uses baseplug, so to build it yourself will require being on the nightly toolchain before building:

```
rustup default nightly
cargo build
```


## Credits

This could not have existed without [baseplug](https://github.com/wrl/baseplug) and the extremely well made buttplug.io, so if you want to see more software like this in the future consider contributing to them.
