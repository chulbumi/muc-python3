#!/bin/sh
while [ 1 ]
do
~/bin/python2.7 ~/bin/twistd -n  --reactor=epoll -y server.py
sleep 1
done
