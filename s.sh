#!/bin/sh
export PYTHONPATH=/home/muc/py3MUC
while [ 1 ]
do
#~/bin/python2.7 ~/bin/twistd -n  --reactor=epoll -y server.py
python3 ~/.local/bin/twistd -n  --reactor=epoll -y server.py
#~/pypy/bin/pypy ~/pypy/bin/twistd -n  --reactor=epoll -y server.py
sleep 1
done
