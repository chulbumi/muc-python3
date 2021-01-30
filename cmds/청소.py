# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
            
        import gc
        gc.enable()
        cnt = gc.collect(1)
        ob.sendLine('* 청소중입니다. %d' % cnt)
