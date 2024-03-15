# ioc 
This project was born out of a desire to learn Rust and to build a [better robocar](https://www.youtube.com/watch?v=qssUHQXRZPk). This project aims to be easy to use, robust and configurable.

#### Running
```shell
ioc config.yml
```
Where `config.yml` defines the inputs, outputs, controllers and the connectivity between them. 

#### Features 
- **core** 
These are core types that are not gated by any cargo features.
    - fundamental data types: `Float`,`Bool` and `String`
    - `Input`s can provide an `InputSource` that emits messages of a fundamental data type. 
    - `Output`s can provide an `OutputSource` that consumes messages of a fundamental data type. 
    - `Channel`s that function as inputs and outputs. Data written to the channel's sink are emitted on the channel's source. 
    - [PID](https://en.wikipedia.org/wiki/Proportional%E2%80%93integral%E2%80%93derivative_controller) controller with variable parameters
    - Simulations in the `sim` module. Currently only a damped harmonic oscilator.

- **wsserver**
This feature is enabled by default. The web socket server is based on [tokio](`https://crates.io/crates/tokio`) and [axum](https://crates.io/crates/axum). Inputs are single valued _per server instance_. Inputs start at a value until updated by a websocket message. The server reports all input and output changes to all connected websockets. Can also serve static content (like a UI).

- **rpi**
This feature must be explicitly enabled when building with the `--features rpi` flag. It can only be built for Raspberry Pi targets and is based on [rppal](https://crates.io/crates/rppal). There is much more to be done here, but for now includes:
    - `RpiDigitalBool` input reads from a raw gpio pin. Pulling the pin high to 3.3v emits a `true` and pulling the pin low to 0v emits `false`.
    - `RpiPwmFloat` output consumes values in [0,1] and controls the PWM duty cycle on the raw gpio pin. Currently, this is only using soft-PWM.

#### Other known "features"
- There is no authentication on the wsserver whatsoever.
- Multiple websockets can connect and fight over the input values. 
- The damped harmonic oscilator simulation can be numerically unstable. 

#### Building 

###### Locally
Requires `cargo`, which you can install with [rustup](https://rustup.rs/)
```shell
cargo build -r
```
###### Cross-build
Example using [cross](https://github.com/cross-rs/cross?tab=readme-ov-file#installation) to build for Raspberry Pi V1
```shell
cross build --target arm-unknown-linux-gnueabihf --features rpi -r
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
- Genericize other devices that work with i2c or spi. There are tons of [embedded-hal crates](https://crates.io/search?q=embedded-hal) that support specific devices like acceleromers and ADCs (inputs), motor controllers and displays (outputs). It should be easy to adapt them for ioc so they work in any build that supports i2c/spi. 
- Other kinds of data streams like audio/video.
- Other possible communication protocols: protobuf, WebRTC
- More targets: support other single-board computers, microcontrollers, maybe even web assembly.
- More control algorithms, improved simulations and developer experience.
