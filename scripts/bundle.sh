#!/bin/bash

rm -rf buttplug_monitor.vst/
$(dirname "$0")/macos_bundler.sh buttplug_monitor $( cd "$(dirname "${BASH_SOURCE[0]}")" ; cd .. ; pwd -P )/target/release/libbuttplug_monitor.dylib