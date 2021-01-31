# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split()
        if line == '':
            target = ob.env
        else:
            target = ob.env.findObjName(line)
        if target == None:
            target = ob.findObjName(line)
            if target == None:
                ob.sendLine('☞ 그런 대상이 없어요!')
                return
        msg = ''
        l = list(target.attr.keys())
        l.sort()
        for at in l:
            msg += '#%s\r\n' % at
            attr = target.attr[at]
            if type(attr) == list:
                msg += '\r\n'.join(attr) + '\r\n'
            else:
                msg += str(attr) + '\r\n'
            """
            for m in str(target.attr[at]):
                msg += ':%s\r\n' % m
            """
            msg += '\r\n'
            
        ob.sendLine(msg)
        
