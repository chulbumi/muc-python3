# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        ob['체력'] = ob.getMaxHp()
        ob['내공'] = ob.getMaxMp()
        ob.sendLine('* 회복되었습니다.')
