# Troubleshooting

## Issues while building from source
One of the [extractors](extractors.md) which bombardier supports is `xpath`.  
Unfortunately Rust does not have a good builtin library to support xpaths. Few out there work specifically with XMLs and do not support HTML parsing.  
  
As this tool can also be used for website performance testing its necessary to support xpath extractor for HTML.  
The only library which supports that right now is [rust-libxml](https://github.com/KWARC/rust-libxml) which is a rust wrapper over C based library [libxml2](http://xmlsoft.org/)  
And this is where building bombardier gets a little painful on some platforms.  

### Linux
Ideally you should not face any issues on linux and code should just build with `cargo build` command.  
`libxml2` is generally available by default on linux based systems. 
In case its not availble please use respective package managers of your linux flavour and install `pkgconfig` & `libxml2dev` 

### MacOS
MacOS may or may not have `libxml2` installed but it can be done using `homebrew`, you also need a lib called `pkg-config`. 
  
Run below commands to install both and then you should be good with your `cargo build`.  
```
brew install pkg-config libxml2
export PKG_CONFIG_PATH="/usr/local/opt/libxml2/lib/pkgconfig"
```

### Windows
Here is where the fun begins. There is no easy way to get this library on windows and even if you get one it cannot be used with `link.exe`.  
  
`libxml2` on windows is built with `gnu` compiler and if you try to build your code with VS build tools it won't work.  
  
If you still want to go ahead and build bombardier executable on windows, follow the below steps. 
- Install [Msys2](http://www.mingw.org/wiki/MSYS). This will also get you `Mingw64`. Use **Mingw64 shell** for below steps.  
- Install `mingw-w64-x86_64-toolchain` & `mingw-w64-x86_64-libxml2`  
  `pacman --noconfirm -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-libxml2`
- Add following package config path variable  
  `export PKG_CONFIG_PATH = \usr\lib\pkgconfig`
- Add below lines to `~/.cargo/config` file. If this file doesn't exist, create one.  
  ```
  [target.x86_64-pc-windows-gnu]
  linker = x86_64-w64-mingw32-gcc.exe
  ar = x86_64-w64-mingw32-gcc-ar.exe
  ```
- Finally, run the below command to generate your executable  
  `cargo build --release --target=x86_64-pc-windows-gnu`
- If everything goes well, you will find `bombardier.exe` under `./target/x86_64-pc-windows-gnu/release` folder  
- Although built successfully, it won't run because it still needs those libraries at runtime
- Copy below `.dlls` from mingw64's `/usr/bin` folder into `release` folder alongside `bombardier.exe`  
  ```
  libiconv-2.dll
  liblzma-5.dll
  libxml2-2.dll
  zlib1.dll
  ```

Note: Bombardier is not officially supported on windows, but you can always try to build on your own