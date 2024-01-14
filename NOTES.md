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

```