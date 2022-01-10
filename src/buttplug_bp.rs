#![allow(incomplete_features)]
#![feature(generic_associated_types)]
//#![feature(async_closure)]
//#![no_std]

extern crate baseplug;
extern crate buttplug;
extern crate serde;

use std::sync::atomic;
use baseplug::{Plugin, ProcessContext};
use serde::{Deserialize, Serialize};
use tokio::{self, runtime::Runtime, sync::mpsc};
use rustfft::{FftPlanner, num_complex::Complex32};

const FFT_SIZE: usize = 4096;
//const MAX_CHANNELS: usize = 2;

mod buttplug_client;

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct ButtplugModel {
        #[model(min = -90.0, max = 3.0)]
        #[parameter(name = "gate cutoff", unit = "Decibels",
            gradient = "Power(0.15)")]
        bass_cutoff: f32,

        #[model(min = 0.0, max = 200.0)]
        #[parameter(name = "low frequency", unit = "Generic",
            gradient = "Power(2.0)")]
        low_freq: f32,

        #[model(min = 0.0, max = 200.0)]
        #[parameter(name = "high frequency", unit = "Generic",
            gradient = "Power(2.0)")]
        high_freq: f32,
    }
}

/*impl ButtplugModel {
    const SEND_RATE: f32 = 20.0; // Per second
}*/

impl Default for ButtplugModel {
    fn default() -> Self {
        Self {
            // "bass_cutoff" is converted from dB to coefficient in the parameter handling code,
            // so in the model here it's a coeff.
            // -0dB == 1.0
            bass_cutoff: 0.8f32,
            low_freq: 20.0f32,
            high_freq: 60.0f32,
        }
    }
}

#[allow(dead_code)] // Runtime is never accessed, storing it so it doesn't delete my futures >:
struct ButtplugMonitor {
    tkrt: Runtime,
    bpio_sender: mpsc::Sender<f32>,

    // FFT stuff
    fft_buffer: [Complex32; FFT_SIZE],
    current_fft: atomic::AtomicUsize,
}

impl Plugin for ButtplugMonitor {
    const NAME: &'static str = "Buttplug Monitor";
    const PRODUCT: &'static str = "Buttplug Monitor";
    const VENDOR: &'static str = "Fractalysoft";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = ButtplugModel;

    #[inline]
    fn new(_sample_rate: f32, _model: &ButtplugModel) -> Self {
        let (tkrt, sender) =
            buttplug_client::start_buttplug_thread(20.0).expect("Could not start Buttplug thread");

        ButtplugMonitor {
            tkrt,
            bpio_sender: sender,

            fft_buffer: [Complex32::new(0.0, 0.0); FFT_SIZE],
            current_fft: atomic::AtomicUsize::new(0),
        }
    }

    #[inline]
    fn process(&mut self, model: &ButtplugModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;

        // If the complex buffer will be overfilled after this, do the FFT
        if self.current_fft.load(atomic::Ordering::Relaxed) + ctx.nframes > FFT_SIZE {
            // v Give this a scratch buffer so it doesn't have to reallocate every time
            FftPlanner::new().plan_fft_forward(FFT_SIZE).process(&mut self.fft_buffer);

            // Get the bins we're interested in
            let low_freq = model.low_freq[ctx.nframes-1];
            let high_freq = model.high_freq[ctx.nframes-1];
            let low_bin = (low_freq / ctx.sample_rate * FFT_SIZE as f32).round() as usize;
            let high_bin = (high_freq / ctx.sample_rate * FFT_SIZE as f32).round() as usize;

            //log::info!("Low bin: {}    High bin: {}", low_bin, high_bin);

            // Get the highest amplitude so we can normalize the FFT
            let mut max_amplitude = 0.0f32;
            for i in 1..FFT_SIZE/2 {
                let amplitude = self.fft_buffer[i].norm();
                if amplitude > max_amplitude {
                    max_amplitude = amplitude;
                }
            }

            // Get the highest amplitude of all the bins we care about
            let mut bass_amplitude = 0.0f32;
            let mut bass_index = 0;
            for i in low_bin..high_bin {
                let amplitude = self.fft_buffer[i].norm();
                if amplitude > bass_amplitude {
                    bass_amplitude = amplitude;
                    bass_index = i;
                }
            }

            //log::info!("Max amplitude: {} at {}", max_amplitude / ctx.nframes, max_index);
            
            // Way too much code just to put the vibration intensity into (0, 1) with 0.05 being lowest
            let bp_max = high_bin as i32 - low_bin as i32;
            let bp_level;
            if bass_amplitude / max_amplitude < model.bass_cutoff[ctx.nframes-1] { // Silence the bass if it is too relatively quiet
                bp_level = 0.0f32;
            }
            else if bass_index < low_bin { // Silence if the bass is too quiet
                bp_level = 0.0f32;
            }
            else if bp_max <= 0 { // Silence if there are no bass bins
                bp_level = 0.0f32;
            }
            else {
                bp_level = 0.05f32 + 0.95f32 * (bass_index as i32 - low_bin as i32) as f32 / bp_max as f32;
            }

            // Will not block
            let _ = self.bpio_sender.try_send( bp_level );

            self.current_fft.store(0, atomic::Ordering::Relaxed);
        }

        // Store in the FFT buffer for later
        let starting_index = self.current_fft.load(atomic::Ordering::Relaxed);

        for i in 0..ctx.nframes {
            output[0][i] = input[0][i];
            output[1][i] = input[1][i];

            //Store in the FFT buffer
            self.fft_buffer[starting_index + i].re = input[0][i] + input[1][i] / 2.0f32;
            self.fft_buffer[starting_index + i].im = 0.0f32;
        }

        // Increment the current FFT index
        self.current_fft.fetch_add(ctx.nframes, atomic::Ordering::Relaxed);

        //let fft_index = self.current_fft.load(atomic::Ordering::Relaxed);
        //log::info!("FFT index: {}, should've written {} samples", fft_index, ctx.nframes);
        
    }
}

baseplug::vst2!(ButtplugMonitor, b"FRbm");
