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
        s = MUGONG[words[1]]
        if s == '':
            for sName in MUGONG.attr:
                if sName.find(words[1]) == 0:
                    s = MUGONG[sName]
                    break
        if s == '':
            ob.sendLine('☞ 그런 무공을 습득한 적이 없습니다.')
            return
        if s.name in target.skillList:
            ob.sendLine('☞ 이미 무공을 익히셨는걸요')
            return
        target.skillList.append(s.name)
        ob.sendLine('☞ 무공이 전수되었습니다.')
