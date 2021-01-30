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
        for m in MUGONG.attr:
            msg += '%14s,'% m
            c += 1
            if c % 5 == 0:
                msg += '\r\n'
        ob.sendLine(msg)
