open terminal from project root directory where Cargo.toml file located
to run project on stm32f3discovery hold RESET (black) button on board and run next command
```
openocd
```

when openocd server says? that it started listening to board on certain port, continue
else stop openocd and repeat procedure.

leave server running and open another terminal instance in the same root folder
run following command there
```
cargo run --release
```

this will build project in release mode and connect to openocd server with gdb connection
after this step program has loaded onto board and you can disconnect it from debug usb port
