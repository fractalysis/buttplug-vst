#![allow(incomplete_features)]
#![feature(generic_associated_types)]
//#![feature(async_closure)]
//#![no_std]

extern crate baseplug;
extern crate buttplug;
extern crate serde;

//use core::panic::PanicInfo;
use baseplug::{Plugin, ProcessContext};
use serde::{Deserialize, Serialize};
use tokio::{self, runtime::Runtime, sync::mpsc};
use rustfft::{FftPlanner, num_complex::Complex32};


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
        }
    }

    #[inline]
    fn process(&mut self, _model: &ButtplugModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;

        //let num_chunks = 2;

        //let chunks = [0..ctx.nframes / 2, ctx.nframes / 2..ctx.nframes];
        let chunks = [0..ctx.nframes];

        for chunk in chunks.iter() {

            for c in 0..1 {
                // v This vec! call has a malloc?, get rid of it
                let mut b = vec![Complex32{re: 0.0, im: 0.0}; chunk.len()];

                //Copy input to output, and also the complex array
                for i in chunk.clone() {
                    output[c][i] = input[c][i];

                    b[i-chunk.start].re = input[c][i];
                }

                FftPlanner::new().plan_fft_forward(ctx.nframes).process(&mut b);
                
                // Get the max amplitude of the frequency area we're interested in
                let low_freq = 20.0f32;
                let high_freq = 50.0f32;
            }

            // Will not block
            let _ = self.bpio_sender.try_send(input[0][0]);

        }
    }
}

baseplug::vst2!(ButtplugMonitor, b"FRbm");
