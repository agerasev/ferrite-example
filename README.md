# ferrite-example

Example of [EPICS](https://epics-controls.org/) IOC using [ferrite-epics](https://github.com/agerasev/ferrite-epics) with [ferrite-rs](https://github.com/agerasev/ferrite-rs).

This IOC contains some PVs of different types.
Output PVs are simply connected to input PVs of the same type that means when you write to output PV the value written appears in the corresponding input PV.

## Prepare

### EPICS

At first you need to get and build [epics-base](https://github.com/epics-base/epics-base). See EPICS documentation for instructions.

After that edit `EPICS_BASE` variable in the following files:

+ `configure/RELEASE`
+ `scripts/env.sh`

to contain actual path to epics-base.

### Rust

You need to have `rustc` and `cargo` installed. The easiest way to get them is to use [rustup](https://rustup.rs/).

## Clone

```sh
git clone https://github.com/agerasev/ferrite-example.git
cd ferrite-example
git submodule update --init
```

## Build

### Backend

```sh
cd backend
cargo build
cd ..
```

### IOC

```sh
make
```

## Run

```sh
cd iocBoot/iocExample/
./st.cmd
```

Or you can use `scripts/run.sh` script to build and run IOC. 

## Test

To automatically test the IOC:

+ Run IOC itself
+ Run `scripts/test.sh` script without stopping the IOC. 
