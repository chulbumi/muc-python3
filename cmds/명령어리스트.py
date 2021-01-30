# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        msg = ''
        c = 0
        for m in ob.cmdList:
            try:
                if ob.cmdList[m].level == 1000:
                    pass
            except:
                continue
            if ob.cmdList[m].level != 1000:
                continue
            msg += '%20s'% m
            c += 1
            if c % 3 == 0:
                msg += '\r\n'
        ob.sendLine(msg)
