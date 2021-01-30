# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.hangul import *

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
            
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [대상] 죽여') #
            return
        
            
        #from lib.object import find_obj, is_mob
        mob = ob.env.findObjName(line)

        if mob == None:
            ob.sendLine('☞ 강호에는 공격할 수 있는것과 없는것이 있지!')
            return

        if not is_mob(mob):
            ob.sendLine('☞ 강호에는 공격할 수 있는것과 없는것이 있지!')
            return

        if mob.get('공격금지'):
            ob.sendLine('☞ 강호에는 공격할 수 있는것과 없는것이 있지!')
            return

        mob.die(ob['이름'])
        #mob.act = ACT_DEATH

        #ob.sendLine('당신은 ' + mob.get('이름') + han_obj(mob.get('이름')) + \
        #    ' 공격하기 시작합니다.')
