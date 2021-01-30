# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if len(line) == 0:
            ob.sendLine('사용법: [몹 이름] 몹제거')
            return

        mob = ob.env.findObjName(line)
        if mob == None or is_mob(mob) == False:
            ob.sendLine('그런 몹이 없어요!')
            return
        ob.env.objs.remove(mob)
        del mob
        ob.sendLine('몹이 제거되었습니다.')
