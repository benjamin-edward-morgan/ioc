# ioc
input. output. control. This is a simple framework for building robots and remotely operated devices. Inputs could be sensors or directly supplied from a user interface. Outputs could be actuators or status indicators. Transformers consume inputs and create new inputs. A configuration file describes a graph that dictates how these elements are connected together in a running system. 

** SEE ALSO: [littlefoot](https://github.com/benjamin-edward-morgan/littlefoot) **: An archive of an RC car powered by IOC build and the web UI for the RC car. 

#### Status
This project is a work-in-progress and under active development. Please get in touch if you are interested in contributing! I will also probably rename this at some point, since IOC as already taken on crates.io

#### Demo
You can build and run the demo in [Docker](https://www.docker.com) without installing other tools. This will take several minutes to build on the first run. Add a `--build` argument to force rebuilding.
```shell
docker compose up -d
```
And visit `localhost:8080` in a browser to view the demo ui. (has problems in Safari at the moment)

Clean it up with
```shell
docker compose down
```

See [other dev notes](NOTES.md)

#### Subprojects

##### ioc
This is the main ioc application, developed in Rust. It supports building for Raspberry Pi and implements simple inputs and outputs for the pi's GPIO, as well as a small set of other devices. See the [ioc README](ioc/README.md).

##### ioc-demo-ui
This is a demo web UI for interacting with ioc's websocket server (if enabled). When the websocket server is enabled, input values can be directly manipulated by a user in real time. Output values are also emitted on the websocket in real time. See the [ioc-demo-ui README](ioc-demo-ui/README.md).
