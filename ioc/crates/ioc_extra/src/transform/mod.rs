
///Takes a Float input that can be positive or negative and emits 'forward' and 'reverse' inputs depending on the sign of the input
pub mod hbridge;

///Takes a Float input (x) and applies a simple linear transform, providing an input that emits m*x+b for some m and b.
pub mod linear;

///An implementation of a PID controller. Takes a set_point, process_var, P, I and D parameters. Provides a single input that emits an output value.    
pub mod pid;

///Various simple 1-1 functions 
pub mod function;

///Used to limit the value or rate-of-change
pub mod limiter;

///Use to average the value over fixed windows
pub mod average;