# S3 rusoto - minio demo
Uploads a file in parts and retrieves it again.

Before you begin, generate a random file, `my-dfu.elf`:
```
$ head -c 256K </dev/urandom > my-dfu.elf
```

Then, run Minio:

```
$ docker compose up
```

Open the Minio console in browser (127.0.0.1:9001). Username: `minioadmin`, password: `minioadmin`.
Create a new Access Key `test` with secret `testtest`, as specified in the `.env` file in this project.

Then, either run the standard demo
```
cargo run --bin standard
```

or the multipart demo (which does not work at the time of writing, but anyway):

```
cargo run --bin multipart
```