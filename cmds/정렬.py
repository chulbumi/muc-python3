# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    level = 1000
    def cmd(self, ob, line):
        #if getInt(ob['관리자등급']) < 1000:
        #    ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
        #    return
        words = line.split()

        if line == '' or len(words) < 2:
            ob.sendLine('☞ 사용법: [보관함] [특성치] 정렬')
            return

        w = words[0]

        obj = ob.env.findObjName(line)
        if obj == None:
            ob.sendLine('☞ 당신의 안광으로는 그런것을 볼수 없다네')
            return

        if is_box(obj) == False:
            ob.sendLine('☞ 당신의 안광으로는 그런것을 볼수 없다네')
            return
         
        self.k = words[1]
        if self.k not in ['힘', '민첩성', '맷집', '명중', '회피', '필살', '운', '방어력', '체력', '내공', '이름']:
            ob.sendLine('☞ 힘|민첩성|맷집|명중|회피|필살|운|방어력|체력|내공|이름 만 가능합니다.')
            return

        if ob['은전'] < 100000:
            ob.sendLine('☞ 은전이 부족해요.')
            return

        #obj.objs.sort(reverse=True, key=self.getOp)
        if self.k == '이름':
            obj.objs.sort(key=lambda item: (item['이름'], item['이름']))
        else:
            obj.objs.sort(key=lambda item: (self.getOp(item), item['이름']))
        ob.sendLine('☞ 정렬되었습니다.')
        ob['은전'] = ob['은전'] - 100000
        return
            

    def getOp(self, obj):
        op = obj.getOption()
        if op == None:
            return 0
        if self.k not in op:
            return 0
        return op[self.k]

