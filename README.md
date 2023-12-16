# Screenshot capturer

A dead simple screenshot capturer script written in Rust. Run it, press `PrtScr`, observe your screenshot in the the `screens` directory (the directory will be created automatically if it does not exist).

Dependencies:

* Chrono
* RDev
* Screenshots

## Overriding directory name

To override the screenshots directory, simply provide the desired name when running the program:

```
kruk_screenshots custom_dir
```

## Building

```
cargo build --release
```

## License

(c) [Ilya Krukowski](http://bodrovis.tech/), licensed under the [beer-ware license](https://fedoraproject.org/wiki/Licensing/Beerware).