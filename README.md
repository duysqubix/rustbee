# Rustbee *an Xbee Rust Library*
[![Build Status][travisimg]][travislink] [![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0) 



*Rustbee* is a library that allows for interaction with XBee supported devices. This work is not part of the official repository for XBee APIs by [Digidotcom][digicom]. 
## Current Status

The codebase is currently in *alpha* stage . Majority of development is done on the `master` 
branch. As it stands this work supports only type of XBee device (S3B Pro 900Mhz), because I am busy with a project where I am utilizing those devices.

Check out the [examples][src_examples] folder for API usage. In order for this to work correctly, you must have `API=1` enabled on XBee device. At the moment, only
the following API frames are supported:

* Transmit Request
* Transmit Status
* AtCommand Frame
* AtCommand Response
* Remote AtCommand Frame
* Remote AtCommand Response



## Contributions
Hopefully if this gets large enough, Digidotcom will take notice and help with official support, but until then it would be greatly appreciated to ask for help from the
community to add to this project and create a usable and stable Rust API for XBee devices. Also do checkout [CONTRIBUTE.md][contribute]

Any questions or information, we welcome you at our [discord][discord] server. Come on by.

## ToDo

* Create logic to handle arbitrary amount of devices when __discovering__ new nodes on the network. 
* Handle different kinds of XBee devices (Wifi, Celluar, v2, etc..)
* Possbily make this async safe? Right now implementation is all sync.


[travisimg]: https://travis-ci.org/duysqubix/rustbee.svg?branch=master
[travislink]: https://travis-ci.org/duysqubix/rustbee
[discord]: https://discord.gg/6arV5Es
[digicom]: https://github.com/digidotcom
[contribute]: https://github.com/rustbee/blob/master/CONTRIBUTING.md
[src_examples]: https://github.com/duysqubix/rustbee/tree/master/examples
