## What is this?

A VST2 plugin for DAWs like Ableton and FL Studio, or DJ software like VirtualDJ that vibrates compatible bluetooth toys (e.g. Lovense Hush) whenever the plugin detects bass in the audio.

For checking the levels on the subbass when your roommates are asleep, making sure the bass is still on in the booth mix, or just kinda havin a good time 🥴


## Running

[DOWNLOAD HERE](https://github.com/fractalysis/buttplug-vst/releases/)

This VST is assuming there is some kind of Intiface server already running, so install https://intiface.com/desktop/ first, and make sure the server is running. Once you open the plugin, any toys in range should automatically connect, but the behaviour might be unpredictable if there is more than one toy or if the toy doesn't vibrate.

Keep in mind this adds a LOT of delay, so usually I add a plugin after this one that delays the audio by around 200 ms. An audio delay may be added into this plugin at some point as well.

Each song may sound better with different parameters:
### Low / High Frequency
These select the band of frequencies which are able to affect the buttplug, this vst will find the loudest frequency in this band and use it as the vibration speed
### Cutoff
If the loudest frequency in the band is quieter than this value compared to the rest of the song, the vibrator will be silenced


## Building

This VST uses baseplug, so to build it yourself will require being on the nightly toolchain before building:

```
rustup default nightly
cargo build
```


## Bugtesting

To help with finding all the bugs in this plugin, it needs to be run in a variety of daws, with a variety of sample rates and buffer sizes. If you notice any problems, just open an issue or a pull request.


## Credits

This could not have existed without [baseplug](https://github.com/wrl/baseplug) and the extremely well made [buttplug.io](https://buttplug.io), so if you want to see more software like this in the future consider contributing to them.
