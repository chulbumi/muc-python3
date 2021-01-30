# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        
        words = line.split()
        if line == '' or len(words) < 3:
            ob.sendLine('☞ 사용법: [대상] [무공] [성] 성올려')
            return
        #ob.sendLine('☞ 공사중입니다.')
        #return
        words = line.split(None, 2)
        target = ob.env.findObjName(words[0])

        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return

        sung = int(words[2])
        target.skillMap[words[1]] = (sung, 199999)

        ob.sendLine('☞ 값이 설정되었습니다.')
        

