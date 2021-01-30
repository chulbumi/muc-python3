# -*- coding: utf-8 -*-

from objs.cmd import Command
from twisted.internet import reactor

class CmdObj(Command):

    def down(self, ob):
        ob.set('cooltime', 0)
        ob.sendLine('낚시줄에 엄청난것이 걸린것 같다...')
        reactor.callLater(3, self.down1, ob)

    def down1(self, ob):
        ob.set('cooltime', 0)
        ob.sendLine('젠장! 낚시줄이 끊어졌다.')

    def cmd(self, ob, line):
        ob.sendLine('낚시바늘에 미끼를 끼우고 낚시대를 드리웁니다.')
        reactor.callLater(3, self.down, ob)
        return
