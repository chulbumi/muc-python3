# -*- coding: utf-8 -*-

from twisted.internet import protocol, reactor
from twisted.application import service, internet
import gc

from client import Client
from loop import Loop
from objs.config import Config
from objs.skill import Skill
from objs.help import Help
from objs.script import Script
from objs.doumi import Doumi
from objs.emotion import Emotion
from objs.player import init_commands
from objs.nickname import Nickname
from objs.oneitem import Oneitem
from objs.rank import Rank
from objs.guild import Guild
from objs.mob import loadAllMob
from objs.item import loadAllItem
#from twitterThread import PostThread, GetTwitterThread
#from objs.room import loadAllMap

print('\r\n=============================================================')
print('          ☞ 무크 파이썬3 버전 서버를 실행 합니다.')
print('=============================================================')
gc.enable()
init_commands()
Emotion()

#loadAllMap()
loadAllMob()
loadAllItem()
print('=============================================================')
print('          ☞ OBJ 로딩이 완료 되었습니다.')
print('=============================================================')
Loop()
"""
t = PostThread(queue)
t.daemon = True
t.start()

t1 = GetTwitterThread(queue)
t1.daemon = True
t1.start()
"""

factory = protocol.ServerFactory()
factory.protocol = Client
application = service.Application("pyMUC_Server")
server = internet.TCPServer(9999, factory)
server.setServiceParent(application)

reactor.listenTCP(9999, factory)
reactor.run()
