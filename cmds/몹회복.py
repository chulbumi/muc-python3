# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 운영자 명령: [대상] 몹회복')
            return
        obj = ob.env.findObjName(line)
        if obj == None or is_mob(obj) == False:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        obj.hp = obj['체력']
        ob.sendLine('* 회복되었습니다.')
