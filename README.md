# Has Marty Zapped Today?

Simple site for determining if [Marty Bent](https://njump.me/npub1guh5grefa7vkay4ps6udxg8lrqxg2kgr3qh9n4gduxut64nfxq0q9y6hjy) has zapped yet today.

## Development

Ensure Rust and Cargo are installed. The easiey way to do that is using [rustup](https://rustup.rs/). Then run the development server and open up `localhost:8000`.

```shell
cargo run
```

## Building and Deployment

To build the application, use `cargo` like normal. When deploying, the expectation is the `assets` directory is also available. This can be made simpler by running the available build script.

```shell
./bin/build.sh
```

### Deploying

An example setup for deploying on a DigitalOcean server is listed below:

```shell
# assume currently on the DO server and in the root of the project
domain=hasmartyzapped.today
./bin/build.sh
adduser --disabled-password martyzaps
cp -r build /home/martyzaps/hasmartyzappedtoday
chown -R martyzaps:martyzaps /home/martyzaps/hasmartyzappedtoday
cat > /lib/systemd/system/hasmartyzappedtoday.service <<EOF
[Unit]
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=martyzaps
Group=martyzaps
WorkingDirectory=/home/martyzaps/hasmartyzappedtoday
PIDFile=/run/hasmartyzappedtoday.pid
ExecStart='/home/martyzaps/hasmartyzappedtoday/hasmartyzappedtoday'
Environment=MARTY_SERVER__DOMAIN=https://$domain
Environment=RUST_LOG=hasmartyzappedtoday=debug

[Install]
WantedBy=default.target
EOF
cat > /etc/nginx/sites-available/hasmartyzappedtoday <<EOF
server {
        listen 80;
        listen [::]:80;

        server_name $domain;

        location / {
                proxy_set_header X-Real-IP $remote_addr;
                proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                proxy_set_header Host $host;
                proxy_set_header X-NginX-Proxy true;
                proxy_pass http://localhost:8000/;
        }
}
EOF
ln -s /etc/nginx/sites-available/hasmartyzappedtoday /etc/nginx/sites-enabled/hasmartyzappedtoday
systemctl restart nginx
certbot --nginx -d hasmartyzapped.today
```

## Support

PRs are more than welcome! I don't know how much more needs to be added, but I'm open to ideas.

Feeling generous? Leave me a tip! ⚡️w3irdrobot@vlt.ge.

Think I'm an asshole but still want to tip? Please donate [to OpenSats](https://opensats.org/).

Want to tell me how you feel? Hit me up [on Nostr](https://njump.me/rob@w3ird.tech).

## License

Distributed under the AGPLv3 License. See [LICENSE.txt](./LICENSE.txt) for more information.
