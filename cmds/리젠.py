# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            c = 0
            for obj in ob.env.objs:
                if is_mob(obj) == False:
                    continue
                if obj.act == ACT_DEATH or obj.act == ACT_REGEN:
                    c += 1
                    obj.doRegen()
            if c != 0:
                #ob.sendLine('☞ 리젠되었습니다.')
                ob.env.printPrompt(ex = ob['이름'])
                ob.sendLine('\r\n\r\n☞ 리젠되었습니다.')
            else:
                ob.sendLine('☞ 리젠될 몹이 없어요!!')
            return
        obj = ob.env.findObjName(line)
        if obj == None or is_item(obj) or is_player(obj):
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if obj.act != ACT_DEATH:
            ob.sendLine('☞ 시체만 가능합니다. *^_^*')
            return
        obj.doRegen()
        ob.env.printPrompt(ex = ob['이름'])
        
 