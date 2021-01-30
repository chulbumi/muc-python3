# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [내용] 표현(\')')
            return
        if ob.env.noComm():
            ob.sendLine('☞ 이지역에서는 어떠한 통신도 불가능합니다.')
            return
        ob.sendLine('당신이 ' + line)
        ob.sendRoom(ob['이름'] + han_iga(ob['이름']) + ' ' + line)

