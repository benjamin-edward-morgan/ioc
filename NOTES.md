#### Setup a systemd service in bookworm +

Defne the Unit for systemd: 

###### /lib/systemd/system/myservice.service
```
[Unit]
Description=myservice description
After=network.target

[Service]
WorkingDirectory=/home/pi
ExecStart=/home/pi/ioc config.yml
Restart=always
User=pi

[Install]
WantedBy=multi-user.target
```

Start service:
```shell
sudo systemctl start myservice.service
```

Enable on startup:
```shell
sudo systemctl enable myservice.service
```

Check the logs:
```shell
journalctl -f | grep myservice
```


#### IDE notes
In VS Code with the rust-analyzer extension, it is helpful to add the following to `settings.json`
```
{
    "rust-analyzer.cargo.features": [
        "rpi"
    ]
}
```