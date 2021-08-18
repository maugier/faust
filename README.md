# Faust - Fast Async URL Status

Faust issues HEAD requests to a potentially large number
of HTTP URLs, and outputs the HTTP response codes in TSV format.

Faust is a tiny Rust program (<100 lines) built on top of [reqwest] and [tokio]. 
It can easily scale to tens of thousands of parallel requests, and process thousands of requests per second on even a modest machine.

## Building

If you have Rust and Cargo installed:

```
$ cargo build --release
$ cp target/release/faust ~/.local/bin
```

## Running

```
$ time faust <<EOF
> http://www.google.com
> https://www.google.com
> http://github.com
> https://github.com
> http://cloudflare.com
> https://cloudflare.com
EOF
http://cloudflare.com/  301 https://www.cloudflare.com/
http://github.com/      301 https://github.com/
http://www.google.com/  200 OK
https://cloudflare.com/ 301 https://www.cloudflare.com/
https://github.com/     200 OK
https://www.google.com/ 200 OK

real    0m0.100s
user    0m0.005s
sys     0m0.009s
```

## Caveats

Faust easily scales to tens of thousands of connections, but you need to adjust the maximum file descriptor limit accordingly:

```
$ ulimit -n 65536
```

If you are behind NAT, such a large number of connections may crash cheap home routers or severely degrade their performance.

[reqwest]: https://docs.rs/reqwest
[tokio]:   https://tokio.rs
