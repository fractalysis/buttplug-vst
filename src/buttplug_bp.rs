#![allow(incomplete_features)]
#![feature(generic_associated_types)]
//#![feature(async_closure)]
//#![no_std]

extern crate baseplug;
extern crate buttplug;
extern crate serde;

use std::sync::{atomic, Arc};
use baseplug::{Plugin, ProcessContext};
use serde::{Deserialize, Serialize};
use tokio::{self, runtime::Runtime, sync::mpsc};
use rustfft::{FftPlanner, num_complex::Complex32};

const FFT_SIZE: usize = 4096;
const MAX_CHANNELS: usize = 2;

mod buttplug_client;

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct ButtplugModel {
        #[model(min = -90.0, max = 3.0)]
        #[parameter(name = "gain", unit = "Decibels",
            gradient = "Power(0.15)")]
        gain: f32,
    }
}

/*impl ButtplugModel {
    const SEND_RATE: f32 = 20.0; // Per second
}*/

impl Default for ButtplugModel {
    fn default() -> Self {
        Self {
            // "gain" is converted from dB to coefficient in the parameter handling code,
            // so in the model here it's a coeff.
            // -0dB == 1.0
            gain: 0.0,
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
    fn process(&mut self, _model: &ButtplugModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;

        // If the complex buffer will be overfilled after this, do the FFT
        if self.current_fft.load(atomic::Ordering::Relaxed) + ctx.nframes > FFT_SIZE {
            FftPlanner::new().plan_fft_forward(FFT_SIZE).process(&mut self.fft_buffer);

            // Get the max amplitude of the frequency area we're interested in
            //let low_freq = 20.0f32;
            //let high_freq = 50.0f32;

            let mut max_amplitude = 0.0f32;
            let mut max_index = 0;
            for i in 1..20 { //Check the bins from 1 (10hz) to 20 (200hz)
                let amplitude = self.fft_buffer[i].norm();
                if amplitude > max_amplitude {
                    max_amplitude = amplitude;
                    max_index = i;
                }
            }

            //log::info!("Max amplitude: {} at {}", max_amplitude, max_index);

            if max_index > 5 { // If it's not a bass frequency (>50hz), silence it
                max_index = 0;
            }

            // Will not block
            let _ = self.bpio_sender.try_send(max_index as f32 / 5.0f32);

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
