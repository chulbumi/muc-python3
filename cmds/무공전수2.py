# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split()
        if line == '' or len(words) < 2:
            ob.sendLine('☞ 사용법: [대상] [무공이름] 무공전수')
            return
        words = line.split(None, 1)
        target = ob.env.findObjName(words[0])
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
        target.skillList.append(words[1])
        ob.sendLine('☞ 무공이 전수되었습니다.')
