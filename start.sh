#!/bin/sh
while [ 1 ]
do
python2 twistd -n  --reactor=epoll -y server.py
sleep 1
done
