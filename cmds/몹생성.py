# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if len(line) == 0:
            ob.sendLine('사용법: [몹 이름] 생성')
            return

        mob = getMob(line)

        if mob == None:
            ob.sendLine('* 생성 실패!!!')
            return
            

        mob = mob.clone()
        mob.place()
        ob.sendLine('[1;32m* [' + mob.get('이름') + '] 생성 되었습니다.[0;37m')

