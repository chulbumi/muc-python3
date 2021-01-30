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
            ob.sendLine('☞ 사용법: [대상] [키] 값삭제')
            return
        words = line.split(None, 1)
        if words[0] == '방':
            target = ob.env
        else:
            target = ob.env.findObjName(words[0])
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
        try:
            target.attr.__delitem__(words[1])
        except:
            ob.sendLine('☞ 해당 키가 없습니다.')
            return
        ob.sendLine('☞ 값이 삭제되었습니다.')
