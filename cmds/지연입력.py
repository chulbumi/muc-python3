# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [입력글] 지연입력')
            return
        from twisted.internet import reactor
        reactor.callLater(1, ob.do_command, line)
        

