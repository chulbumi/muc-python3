# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
            
        msg = ''
        for index in ONEITEM.attr:
            owner = ONEITEM.attr[index]
            name = ONEITEM.getName(index)
            msg += '%-16s (%-16s) : %s\r\n' % (name, index, owner)
        ob.sendLine(msg)
