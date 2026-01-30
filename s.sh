#!/bin/sh
export PYTHONPATH=/Users/mac/muc-python3
while [ 1 ]
do
#~/bin/python2.7 ~/bin/twistd -n  --reactor=epoll -y server.py
python3 /Users/mac/Library/Python/3.9/bin/twistd -n  -y server.py
#~/pypy/bin/pypy ~/pypy/bin/twistd -n  --reactor=epoll -y server.py
sleep 1
done
