# rustls-server-sni

A simple server that supports https.

This server sends a plaintext response containing sni to every request.

# Test

Add this to "/etc/hosts" file.

```
127.0.0.1   localtest
127.0.0.1   localtestx
```

NOTE: You should probably remove these when you are done to keep your "hosts" file clean :)

Run the program.

```
cargo run
```

Enter `https://localhost:3443/` to browser.
This should work as expected.

Enter `https://localtest:3443/` to browser.
This should work too.

Enter `https://localtestx:3443/` to browser.
This shouldn't. Because it is not listed in `config.json` file.

