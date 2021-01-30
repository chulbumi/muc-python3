# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        #words = line.split()
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [이름] 방이름')
            return
        ob.env['이름'] = line
        ob.env.save()
        ob.sendLine('방이 이름이 변경 되었습니다.')
        
