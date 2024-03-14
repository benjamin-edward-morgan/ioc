# ioc 
This project was born out of a desire to learn Rust and to build a [better robocar](https://www.youtube.com/watch?v=qssUHQXRZPk). This project aims to be easy to use, robust and configurable.

#### Running
```shell
ioc config.yml
```
Where `config.yml` defines the inputs, outputs, controllers and the connectivity between them. 

#### Crates
- `ioc_core` includes fundamental data types used in all other ioc libraries. 
- `ioc_server` is a server for websocket endpoints that allows clients to send and received updated values in real time.
- `ioc_devices` has ioc implementations of various i2c or spi devices, like sensors and acuators.
- `ioc_rpi_gpio` brings in raspberry pi specific bindings. this is required for `ioc_devices`
- `ioc_extra` less-stable collection of other ioc objects 

#### Other known "features"
- There is no authentication on the wsserver whatsoever.
- Multiple websockets can connect and fight over the input values. 

#### Building 

###### Locally
Requires `cargo`, which you can install with [rustup](https://rustup.rs/)
```shell
cargo build -r -p ioc
```
###### Cross-build
Example using [cross](https://github.com/cross-rs/cross?tab=readme-ov-file#installation) to build for Raspberry Pi V1
```shell
cross build --target arm-unknown-linux-gnueabihf --features rpi -r -p ioc
```
The target will depend on the hardware model.
| Raspberry pi model | Target |
| --- | --- |
| A, A+, B, B+, Zero, Zero W | arm-unknown-linux-gnueabihf | 
| 2B, 3A+, 3B, 3B+, Zero 2W | armv7-unknown-linux-gnueabihf |
| 4B, 400, 5 | aarch64-unknown-linux-gnu |

#### Future Work
- Create a "wsclient" feature, analagous to the "wsserver" feature and using the same websocket protocol. One possible senario is: A remote device running ioc connects to a cloud server, also running ioc. A user can connect to the cloud server to interact with the remote device.
- Actual authentication on those websockets. A requirement for the previous item.
- Other possible communication protocols: protobuf, WebRTC
- More targets: support other single-board computers, microcontrollers, maybe even web assembly.
- More control algorithms, improved simulations and developer experience.
- ~~Genericize other devices that work with i2c or spi. There are tons of [embedded-hal crates](https://crates.io/search?q=embedded-hal) that support specific devices like acceleromers and ADCs (inputs), motor controllers and displays (outputs). It should be easy to adapt them for ioc so they work in any build that supports i2c/spi.~~
- ~~Other kinds of data streams like audio/video.~~ ("video" at least)
