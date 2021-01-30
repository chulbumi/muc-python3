# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [모두] 분해')
            return
        words = line.split()

        if len(words) != 1:
            ob.sendLine('☞ 사용법: [모두] 분해')
            return

        mob = ob.env.findMerchant()
        if mob == None:
            ob.sendLine('☞ 상인이 없어요. ^_^')
            return
        if mob['물건구입'] == '':
            ob.sendLine('☞ 상인이 없어요. ^_^')
            return

        if line == '모두':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if '올숙무기' in item.index:
                    continue
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
                if item.inUse == True:
                    continue
                if item['종류'] != '방어구' and item['종류'] != '무기':
                    continue
 
                op = item.getOption()
                if op == None:
                    continue  

                if len(op) >= 4:
                    c += 1
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 분해합니다.' % item.getNameA())
                ob.sendRoom('%s %s 1개를 분해합니다.' % ( ob.han_iga(), item.getNameA()))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
                return
            itm = getItem('강철조각')
            for i in range(c):
                it = itm.deepclone()
                ob.objs.append(it)
        else:
            ob.sendLine('☞ 사용법: [모두] 분해')
            return


