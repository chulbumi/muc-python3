# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        import pickle
        import uuid

        if line == '':
            ob.sendLine('☞ 사용법: [물품번호] 대여')
            return
        num = getInt(line)

        mob = ob.env.findObjName('진영')
        if mob == None:
            ob.sendLine('☞ 이곳에서는 불가능해요.')
            return

        try:
            with open("data/config/book.dat", "rb") as fr:
                data = pickle.load(fr)
        except:
            ob.sendLine('☞ 대여 가능한 물품이 없습니다.')
            return
        if num < 1 or num > len(data):
            ob.sendLine('☞ 대여 가능한 물품이 없습니다.')
            return

        itm = data[num - 1]

        if itm['대여가능'] == False:
            ob.sendLine('☞ 현재 대여중 입니다.')
            return
        itm["대여가능"] = False
        itm["대여"] = ob['이름'] 
        item = getItem(itm["인덱스"])
        item = item.deepclone()
        item.attr = itm["attr"]
        item['고유번호'] = itm['고유번호']
        ob.append(item)

        with open("data/config/book.dat", "wb") as fw:
            pickle.dump(data, fw)
        ob.sendLine('☞ 대여가 완료 되었습니다.')
